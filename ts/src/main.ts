import { Command } from 'commander';
import { generateKeypair, formatKeypair } from './crypto';

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

// カスタムヘルプフォーマット（clap と一致させる）
program.configureHelp({
  formatHelp: () => {
    return `Rad source control management

Usage: rad [COMMAND]

Commands:
  keygen  Generate Ed25519 key pair
  help    Print this message or the help of the given subcommand(s)

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
