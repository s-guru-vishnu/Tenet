#!/usr/bin/env node

const { program } = require('commander');
const { spawn } = require('child_process');
const chalk = require('chalk');
const path = require('path');
const fs = require('fs');

// Find the tenet binary
// It assumes the tenet binary is either globally installed and available in PATH,
// or located relative to this package if used in the monorepo.
function getTenetBinary() {
    // Basic fallback - assume it's in PATH
    return 'tenet';
}

function runTenetCommand(args) {
    const binary = getTenetBinary();
    
    console.log(chalk.gray(`> Executing: ${binary} ${args.join(' ')}`));
    
    const child = spawn(binary, args, {
        stdio: 'inherit',
        shell: true
    });

    child.on('error', (err) => {
        console.error(chalk.red('\nFailed to start TENET process.'));
        console.error(chalk.yellow('Make sure TENET desktop app/CLI is installed globally.'));
        console.error(`Error: ${err.message}`);
        process.exit(1);
    });

    child.on('close', (code) => {
        if (code !== 0) {
            process.exit(code);
        }
    });
}

program
    .name('tenet-cli')
    .description('CLI wrapper for TENET - Time-Travel File System')
    .version('0.1.0');

program
    .command('watch <directory>')
    .alias('w')
    .description('Start watching a directory for changes')
    .action((directory) => {
        console.log(chalk.cyan(`Starting TENET watcher for: ${directory}`));
        runTenetCommand(['watch', directory]);
    });

program
    .command('history <file>')
    .alias('h')
    .description('Show version history for a specific file')
    .option('-l, --limit <number>', 'Maximum number of versions to display', '20')
    .action((file, options) => {
        console.log(chalk.cyan(`Fetching history for: ${file}`));
        runTenetCommand(['history', file, '--limit', options.limit]);
    });

program
    .command('restore <target>')
    .alias('r')
    .description('Restore a file to a previous version (format: file@time)')
    .option('-n, --dry-run', 'Preview the restore without actually modifying the file')
    .action((target, options) => {
        console.log(chalk.cyan(`Restoring target: ${target}`));
        const args = ['restore', target];
        if (options.dryRun) {
            args.push('--dry-run');
        }
        runTenetCommand(args);
    });

program
    .command('status')
    .alias('s')
    .description('Show current tracking status and statistics')
    .action(() => {
        runTenetCommand(['status']);
    });

program.parse();
