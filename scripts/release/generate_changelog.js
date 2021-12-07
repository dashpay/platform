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

const gitRawCommitsOpts = {
  from,
};

const outfile = 'CHANGELOG.md';
const tmp = tempfile();

const readStream = fs.createReadStream(outfile)
  .on('error', function () {
    console.warn('infile does not exist.')
  });

conventionalChangelog(options, undefined, gitRawCommitsOpts)
.pipe(addStream(readStream))
  .pipe(fs.createWriteStream(tmp))
  .on('finish', function () {
    fs.createReadStream(tmp)
      .pipe(fs.createWriteStream(outfile))
  });
