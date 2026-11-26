import os from 'os';
import path from 'path';
import fs from 'fs';
import { create } from 'tar';
import { getConfigSecretValues } from '../config/obfuscateConfig.js';
import hideString from '../util/hideString.js';
import maskOperatorIdentity, { getOperatorIdentity } from '../util/maskOperatorIdentity.js';

/**
 * Config secrets shorter than this are not searched for in the samples.
 *
 * A value that short turns up in ordinary text by chance - a word in a log
 * line, digits in a number - as readily as it marks the secret, and masking it
 * there would mangle the logs the report exists to carry. The secrets dashmate
 * generates and its default passwords are all longer, and the values of the
 * container environment, where a config value of any length would otherwise be
 * copied, are left out when samples are collected.
 */
const MIN_SCRUBBED_SECRET_LENGTH = 8;

/**
 * Mask every config secret long enough to search for wherever it occurs in a
 * string.
 *
 * The config sample has its secret fields hidden already, but the same values
 * reach other samples on their own: a service can log one, an RPC response can
 * echo one. They are all known when the archive is written, so they are taken
 * out of every file of it.
 *
 * A secret that is also the whole value of another config field - a password
 * set to the name of an environment, say - is masked there as well. Doctor may
 * then be unable to read that config back, which is the safer way to fail.
 *
 * @param {Config} config - as stored, not obfuscated
 * @return {function(string): string}
 */
function createSecretScrubber(config) {
  const identity = getOperatorIdentity();

  const needles = [...new Set(
    getConfigSecretValues(config)
      .filter((value) => value.length >= MIN_SCRUBBED_SECRET_LENGTH)
      // Samples have the operator's name masked out as they are collected, so
      // a secret that contains the name is found there in that form
      .flatMap((value) => [value, maskOperatorIdentity(value, identity)])
      // Text that embeds JSON, such as a structured log line, carries a value
      // with a quote or a backslash in it escaped
      .flatMap((value) => [value, JSON.stringify(value).slice(1, -1)])
      // Masking the name can leave too little of a secret to search for
      .filter((value) => value.length >= MIN_SCRUBBED_SECRET_LENGTH),
  )]
    // Longest first, so a secret that contains another is masked whole rather
    // than leaving the part outside the shorter one in clear
    .sort((a, b) => b.length - a.length);

  return (text) => needles.reduce(
    (scrubbed, needle) => scrubbed.replaceAll(needle, hideString(needle)),
    text,
  );
}

function writeSampleFile(archiveDir, service, filename, data, scrub) {
  if (data === undefined || data === null) {
    return;
  }

  const serviceDir = path.join(archiveDir, service ?? '');

  let buffer;
  let filetype;

  const dataType = typeof data;

  if (dataType === 'string') {
    buffer = scrub(data);
    filetype = '.txt';
  } else {
    // Scrubbed value by value rather than as text, so keys, numbers and the
    // structure are left alone and doctor can still read the sample back
    buffer = JSON.stringify(
      data,
      (_key, value) => (typeof value === 'string' ? scrub(value) : value),
      2,
    );
    filetype = '.json';
  }

  if (!fs.existsSync(serviceDir)) {
    fs.mkdirSync(serviceDir);
  }

  fs.writeFileSync(path.join(serviceDir, `${filename}${filetype}`), buffer, 'utf8');
}

/**
 * @param {Samples} samples
 * @param {string} folderPath
 * @param {Config} config - the config the samples were collected from, not obfuscated
 */
export default async function archiveSamples(samples, folderPath, config) {
  if (!config) {
    throw new Error('The config the samples were collected from is needed to remove its secrets');
  }

  const scrub = createSecretScrubber(config);

  const archiveName = `dashmate-report-${samples.date.toISOString()}`;

  // Staged in a fresh directory only the operator can read, and removed once
  // packed: the archive is the one copy meant to leave the machine, and
  // nothing that was in the temp directory before is packed along with it.
  const archiveDir = fs.mkdtempSync(path.join(os.tmpdir(), `${archiveName}-`));

  try {
    writeSampleFile(archiveDir, null, 'date', samples.date.toISOString(), scrub);
    writeSampleFile(archiveDir, null, 'systemInfo', samples.getSystemInfo(), scrub);
    writeSampleFile(archiveDir, null, 'dockerError', samples.getStringifiedDockerError(), scrub);
    writeSampleFile(archiveDir, null, 'dashmateConfig', samples.getDashmateConfig(), scrub);
    writeSampleFile(archiveDir, null, 'dashmateVersion', samples.getDashmateVersion(), scrub);

    for (const [serviceName, service] of Object.entries(samples.getServices())) {
      for (const [key, data] of Object.entries(service)) {
        if (data !== undefined && data !== null) {
          writeSampleFile(archiveDir, serviceName, key, data, scrub);
        }
      }
    }

    // The staging directory's own entry is left out: it would carry the
    // private mode onto whatever directory the archive is extracted into
    await create(
      {
        cwd: archiveDir,
        gzip: true,
        file: path.join(folderPath, `${archiveName}.tar.gz`),
      },
      fs.readdirSync(archiveDir),
    );
  } finally {
    fs.rmSync(archiveDir, { recursive: true, force: true });
  }
}
