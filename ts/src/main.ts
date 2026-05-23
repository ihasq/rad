import { Command } from 'commander';
import { generateKeypair, formatKeypair, keypairFromSecret, formatPublicKey } from './crypto';
import { signOperation, injectSignature } from './sign';
import { verifyOperation } from './verify';
import { RegionMap } from './region';
import { OpLog } from './oplog';
import { handleWrite, handleChain } from './pipeline';
import { handleAccept } from './accept';
import { handleReject } from './reject';
import { handleDelete } from './delete';
import { initProject } from './init';
import { FounderTree } from './founder';
import { RadStore } from './store';
import { createRelayApp } from './relay/app';
import { importFromGit } from './git/import';
import { exportToGit } from './git/export';
import { runStatus } from './cmd/status';
import { runLog } from './cmd/log';
import { runDiff } from './cmd/diff';
import { runClone } from './cmd/clone';
import { runPush } from './cmd/push';
import { runPull } from './cmd/pull';
import * as fs from 'fs';

const program = new Command();
program
  .name('rad')
  .version('rad 0.0.1', '-V, --version', 'Print version')
  .description('Rad source control management')
  .helpOption('-h, --help', 'Print help');

program
  .command('keygen')
  .description('Generate Ed25519 key pair')
  .action(() => {
    const kp = generateKeypair();
    console.log(formatKeypair(kp));
  });

program
  .command('sign')
  .description('Sign an operation from stdin')
  .requiredOption('--secret-key <key>', 'Base64 Ed25519 secret key')
  .action(async (opts) => {
    const input = await readStdin();
    const sig = signOperation(input.trim(), opts.secretKey);
    console.log(injectSignature(input.trim(), sig));
  });

program
  .command('verify')
  .description('Verify a signed operation from stdin')
  .requiredOption('--public-key <key>', 'Base64 Ed25519 public key')
  .action(async (opts) => {
    const input = await readStdin();
    if (verifyOperation(input.trim(), opts.publicKey)) {
      console.log('valid');
    } else {
      console.log('invalid');
      process.exit(1);
    }
  });

program
  .command('region')
  .description('Manage code regions (reads commands from stdin)')
  .action(async () => {
    const input = await readStdin();
    const map = new RegionMap();
    const lines = input.trim().split('\n');
    for (const line of lines) {
      const parts = line.trim().split(/\s+/);
      switch (parts[0]) {
        case 'register': {
          const r = {
            id: parts[1] + ':' + parts[2] + '-' + parts[3],
            filePath: parts[1],
            startLine: parseInt(parts[2]),
            endLine: parseInt(parts[3]),
            ownerId: parts[4],
          };
          if (map.register(r)) {
            console.log('registered: ' + r.filePath + ':' + r.startLine + '-' + r.endLine + ' (owner: ' + r.ownerId + ')');
          } else {
            console.log('ignored: region already registered');
          }
          break;
        }
        case 'owner': {
          const o = map.getOwner(parts[1], parseInt(parts[2]));
          console.log(o ?? 'unowned');
          break;
        }
        case 'list': {
          for (const r of map.list(parts[1])) {
            console.log(r.filePath + ':' + r.startLine + '-' + r.endLine + '\towner:' + r.ownerId);
          }
          break;
        }
        case 'role': {
          console.log(map.getRole(parts[1], parseInt(parts[2]), parts[3]));
          break;
        }
      }
    }
  });

program
  .command('init')
  .description('Initialize a new Rad project')
  .requiredOption('--participant <id>', 'Participant ID')
  .requiredOption('--secret-key <key>', 'Base64 Ed25519 secret key')
  .action((opts) => {
    const kp = keypairFromSecret(opts.secretKey);
    const publicKey = formatPublicKey(kp);

    try {
      const result = initProject('.', opts.participant, publicKey);
      console.log('initialized: .');
      console.log('founder: ' + result.founder);
    } catch (e) {
      console.error('error:', (e as Error).message);
      process.exit(1);
    }
  });

program
  .command('pipeline')
  .description('Execute commands from stdin (region, write, chain)')
  .option('--ephemeral', 'Run in ephemeral mode (in-memory only)')
  .action(async (opts) => {
    const input = await readStdin();
    const opIds: string[] = [];

    // Open RadStore (only if not ephemeral)
    let store: RadStore | null = null;
    if (!opts.ephemeral) {
      try {
        store = RadStore.open('.');
      } catch (e) {
        console.error('error:', (e as Error).message);
        process.exit(1);
      }
    }

    // Load state from store or create new
    const regionMap = new RegionMap();
    const oplog = new OpLog();
    const founderTree = new FounderTree('');

    if (store) {
      regionMap.loadRegions(store.loadRegions());
      oplog.loadOperations(store.loadOplog());
      const founderData = store.loadFounders();
      founderTree.loadFoundersObject(founderData.founders);
    }

    // Helper to expand @N references
    function expandRefs(line: string): string {
      let result = line;
      for (let i = 0; i < opIds.length; i++) {
        result = result.replaceAll('@' + (i + 1), opIds[i]);
      }
      return result;
    }

    const lines = input.trim().split('\n');
    for (const line of lines) {
      const expanded = expandRefs(line.trim());
      const parts = expanded.split(/\s+/);
      switch (parts[0]) {
        case 'write': {
          // write <file> <start> <end> <participant> <secret-key> <text>
          const file = parts[1];
          const participant = parts[4];
          founderTree.registerFromWrite(file, participant);
          // ファイル Founder も登録（最初の write 時のみ）
          founderTree.registerFileFounder(file, participant);

          const output = handleWrite(parts, regionMap, oplog);
          // Extract op-id from JSON output
          try {
            const json = JSON.parse(output);
            if (json.id) {
              opIds.push(json.id);
            }
          } catch {}
          console.log(output);

          // Persist state (only if not ephemeral)
          if (store) {
            store.saveOplog(oplog.getAllOperations());
            store.saveRegions(regionMap.getAllRegions());
            store.saveFounders(founderTree.getFoundersObject());
          }
          break;
        }
        case 'chain': {
          console.log(handleChain(parts, oplog));
          break;
        }
        case 'accept': {
          // accept <op-id> <leader> <secret-key>
          try {
            const result = handleAccept(parts[1], parts[2], regionMap, oplog);
            console.log(JSON.stringify(result));
            if (store) {
              store.saveOplog(oplog.getAllOperations());
            }
          } catch (e) {
            console.error('error:', (e as Error).message);
          }
          break;
        }
        case 'reject': {
          // reject <op-id> <rejecter> <secret-key> ["reason"]
          const reason = parts.length > 4 ? parts.slice(4).join(' ').replace(/^"|"$/g, '') : undefined;
          try {
            const result = handleReject(parts[1], parts[2], reason, regionMap, founderTree, oplog);
            console.log(JSON.stringify(result));
            if (store) {
              store.saveOplog(oplog.getAllOperations());
            }
          } catch (e) {
            console.error('error:', (e as Error).message);
          }
          break;
        }
        case 'region': {
          // region サブコマンドも pipeline 内でサポート
          if (parts[1] === 'register') {
            const r = {
              id: parts[2] + ':' + parts[3] + '-' + parts[4],
              filePath: parts[2],
              startLine: parseInt(parts[3]),
              endLine: parseInt(parts[4]),
              ownerId: parts[5],
            };
            if (regionMap.register(r)) {
              console.log('registered: ' + r.filePath + ':' + r.startLine + '-' + r.endLine + ' (owner: ' + r.ownerId + ')');
              if (store) {
                store.saveRegions(regionMap.getAllRegions());
              }
            } else {
              console.log('ignored: region already registered');
            }
          }
          break;
        }
        case 'founder': {
          // founder [dir]
          const dir = parts[1] || '.';
          // Strip trailing slash for consistency
          const dirNormalized = dir.replace(/\/$/, '') || '.';
          const founder = founderTree.getFounder(dirNormalized);
          if (founder) {
            console.log(dir + ': founder: ' + founder);
          } else {
            console.log(dir + ': no founder');
          }
          break;
        }
        case 'file-founder': {
          // file-founder <file-path>
          if (parts.length < 2) {
            console.error('usage: file-founder <file-path>');
            break;
          }
          const filePath = parts[1];
          const fileFounder = founderTree.getFileFounder(filePath);
          if (fileFounder) {
            console.log(filePath + ': file-founder: ' + fileFounder);
          } else {
            console.log(filePath + ': no file-founder');
          }
          break;
        }
        case 'delete': {
          // delete <file-path> <participant> <secret-key>
          if (parts.length < 4) {
            console.error('usage: delete <file-path> <participant> <secret-key>');
            break;
          }
          const filePath = parts[1];
          const participant = parts[2];
          const secretKey = parts[3];

          try {
            const result = handleDelete(filePath, participant, secretKey, founderTree, oplog);
            console.log(JSON.stringify(result));
            if (store) {
              store.saveOplog(oplog.getAllOperations());
            }
          } catch (e) {
            console.error('error:', (e as Error).message);
          }
          break;
        }
      }
    }
  });

program
  .command('relay')
  .description('Start Rad Relay HTTP server')
  .option('--port <port>', 'Port number', '8787')
  .option('--storage <type>', 'Storage backend: memory | s3', 'memory')
  .option('--s3-endpoint <url>', 'S3 endpoint URL')
  .option('--s3-bucket <name>', 'S3 bucket name')
  .option('--s3-access-key <key>', 'S3 access key')
  .option('--s3-secret-key <key>', 'S3 secret key')
  .option('--s3-region <region>', 'S3 region', 'us-east-1')
  .action(async (opts) => {
    let store;

    if (opts.storage === 's3') {
      // Validate S3 options
      if (!opts.s3Endpoint || !opts.s3Bucket || !opts.s3AccessKey || !opts.s3SecretKey) {
        console.error('error: S3 storage requires --s3-endpoint, --s3-bucket, --s3-access-key, and --s3-secret-key');
        process.exit(1);
      }

      try {
        const { S3Backend } = await import('./storage/s3-backend');
        const { S3RadStore } = await import('./storage/s3-store');

        const s3Backend = new S3Backend({
          endpoint: opts.s3Endpoint,
          bucket: opts.s3Bucket,
          accessKey: opts.s3AccessKey,
          secretKey: opts.s3SecretKey,
          region: opts.s3Region,
        });

        store = new S3RadStore(s3Backend);
        console.log('rad relay using S3 storage: ' + opts.s3Endpoint + '/' + opts.s3Bucket);
      } catch (e) {
        console.error('error: Failed to initialize S3 storage:', (e as Error).message);
        process.exit(1);
      }
    }

    const { app } = await createRelayApp(store);
    const port = parseInt(opts.port);
    console.log('rad relay listening on port ' + port);

    Bun.serve({
      fetch: app.fetch,
      port,
    });
  });

program
  .command('compact')
  .description('Compact operation log into snapshots')
  .action(() => {
    try {
      const store = RadStore.open('.');
      store.compact();
      console.log('compacted');
    } catch (e) {
      console.error('error:', (e as Error).message);
      process.exit(1);
    }
  });

program
  .command('import')
  .description('Import Git history into Rad')
  .action(async () => {
    try {
      const result = await importFromGit('.');
      console.log(`imported: ${result.commitCount} commits → ${result.operationCount} operations`);
      console.log(`participants: ${result.participantCount} registered`);
    } catch (e) {
      console.error('error:', (e as Error).message);
      process.exit(1);
    }
  });

program
  .command('export')
  .description('Export Rad accepted operations to Git')
  .action(async () => {
    try {
      const result = await exportToGit('.');
      console.log(`exported: ${result.operationCount} operations → ${result.commitCount} commits`);
    } catch (e) {
      console.error('error:', (e as Error).message);
      process.exit(1);
    }
  });

program
  .command('status')
  .description('Show project status')
  .action(() => {
    try {
      const store = RadStore.open('.');
      const output = runStatus(store);
      process.stdout.write(output);
    } catch (e) {
      console.error('error:', (e as Error).message);
      process.exit(1);
    }
  });

program
  .command('log')
  .description('Show operation log')
  .option('--participant <id>', 'Filter by participant ID')
  .option('--file <path>', 'Filter by file path')
  .option('--status <status>', 'Filter by status')
  .action((opts) => {
    try {
      const store = RadStore.open('.');
      const output = runLog(store, {
        participant: opts.participant,
        file: opts.file,
        status: opts.status,
      });
      process.stdout.write(output);
    } catch (e) {
      console.error('error:', (e as Error).message);
      process.exit(1);
    }
  });

program
  .command('diff')
  .description('Show diff between accepted and visible writes')
  .action(() => {
    try {
      const store = RadStore.open('.');
      const output = runDiff(store);
      process.stdout.write(output);
    } catch (e) {
      console.error('error:', (e as Error).message);
      process.exit(1);
    }
  });

program
  .command('clone <url>')
  .description('Clone a project from a Relay server')
  .requiredOption('--participant <id>', 'Participant ID')
  .requiredOption('--secret-key <key>', 'Base64 Ed25519 secret key')
  .action(async (url, opts) => {
    try {
      const output = await runClone(url, opts.participant, opts.secretKey);
      process.stdout.write(output);
    } catch (e) {
      console.error('error:', (e as Error).message);
      process.exit(1);
    }
  });

program
  .command('push')
  .description('Push local operations to Relay server')
  .action(async () => {
    try {
      const store = RadStore.open('.');
      const output = await runPush(store);
      process.stdout.write(output);
    } catch (e) {
      console.error('error:', (e as Error).message);
      process.exit(1);
    }
  });

program
  .command('pull')
  .description('Pull remote operations from Relay server')
  .action(async () => {
    try {
      const store = RadStore.open('.');
      const output = await runPull(store);
      process.stdout.write(output);
    } catch (e) {
      console.error('error:', (e as Error).message);
      process.exit(1);
    }
  });

// stdin 読み取りヘルパー
async function readStdin(): Promise<string> {
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) chunks.push(chunk);
  return Buffer.concat(chunks).toString('utf-8');
}

// カスタムヘルプフォーマット（clap と一致させる）
program.configureHelp({
  formatHelp: () => {
    return `Rad source control management

Usage: rad [COMMAND]

Commands:
  keygen    Generate Ed25519 key pair
  sign      Sign an operation from stdin
  verify    Verify a signed operation from stdin
  region    Manage code regions (reads commands from stdin)
  pipeline  Execute commands from stdin (region, write, chain)
  init      Initialize a new Rad project
  relay     Start Rad Relay HTTP server
  compact   Compact operation log into snapshots
  import    Import Git history into Rad
  export    Export Rad accepted operations to Git
  status    Show project status
  log       Show operation log
  diff      Show diff between accepted and visible writes
  clone     Clone a project from a Relay server
  push      Push local operations to Relay server
  pull      Pull remote operations from Relay server
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
`;
  }
});

program.parse(process.argv);

if (!process.argv.slice(2).length) {
  program.outputHelp();
}
