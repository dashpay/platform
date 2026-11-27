const fs = require('fs');
const tempfile = require('tempfile')
const addStream = require('add-stream');
const conventionalChangelog = require('conventional-changelog');

const [ from ] = process.argv.slice(2);

if (!from) {
  console.error('usage: generate_changelog.js v0.22.0');
  process.exit(1);
}

const options = {
  preset: 'dash',
};

// conventional-changelog finds the repository by reading `.git/config` when
// package.json names none. In a git worktree `.git` is a file, so the links
// come out as `///compare/...` and `undefined/undefined/undefined/issues/...`.
// Naming the repository here makes the links the same wherever the release
// is cut from.
const context = {
  host: 'https://github.com',
  owner: 'dashpay',
  repository: 'platform',
  repoUrl: 'https://github.com/dashpay/platform',
};

const gitRawCommitsOpts = {
  from,
};

const outfile = 'CHANGELOG.md';
const tmp = tempfile();

const readStream = fs.createReadStream(outfile)
  .on('error', function () {
    console.warn('infile does not exist.')
  });

conventionalChangelog(options, context, gitRawCommitsOpts)
.pipe(addStream(readStream))
  .pipe(fs.createWriteStream(tmp))
  .on('finish', function () {
    fs.createReadStream(tmp)
      .pipe(fs.createWriteStream(outfile))
  });
