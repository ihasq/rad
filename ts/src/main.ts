import { Command } from 'commander';
import { startRelay } from './relay/server';

const program = new Command();
program
  .name('rad')
  .version('rad 0.0.1')
  .description('Rad Relay server');

program
  .command('relay')
  .description('Start Rad Relay HTTP server (WASM Core)')
  .option('--port <port>', 'Port number', '8787')
  .option('--storage <type>', 'memory | s3', 'memory')
  .option('--s3-endpoint <url>')
  .option('--s3-bucket <name>')
  .option('--s3-access-key <key>')
  .option('--s3-secret-key <key>')
  .option('--s3-region <region>', '', 'us-east-1')
  .option('--wasm <path>', 'WASM Core path', './rad_wasm.wasm')
  .action(async (opts) => { await startRelay(opts); });

program.parse(process.argv);
