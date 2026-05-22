import { Command } from 'commander';
import { generateKeypair, formatKeypair } from './crypto';
import { signOperation, injectSignature } from './sign';
import { verifyOperation } from './verify';
import { RegionMap } from './region';
import { OpLog } from './oplog';
import { handleWrite, handleChain } from './pipeline';

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
  .command('pipeline')
  .description('Execute commands from stdin (region, write, chain)')
  .action(async () => {
    const input = await readStdin();
    const regionMap = new RegionMap();
    const oplog = new OpLog();
    const lines = input.trim().split('\n');
    for (const line of lines) {
      const parts = line.trim().split(/\s+/);
      switch (parts[0]) {
        case 'write': {
          console.log(handleWrite(parts, regionMap, oplog));
          break;
        }
        case 'chain': {
          console.log(handleChain(parts, oplog));
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
            } else {
              console.log('ignored: region already registered');
            }
          }
          break;
        }
      }
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
