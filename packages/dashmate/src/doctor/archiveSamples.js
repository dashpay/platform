import crypto from 'crypto';
import os from 'os';
import path from 'path';
import fs from 'fs';
import { pipeline } from 'stream/promises';
import { create } from 'tar';
import createSecretScrubber from './createSecretScrubber.js';
import getSampleFreeTextFields, { getSampleFactFilter } from './getSampleFreeTextFields.js';

function writeSampleFile(archiveDir, service, filename, data, scrub) {
  const freeTextFields = service ? getSampleFreeTextFields(service, filename) : null;
  const isFact = service ? getSampleFactFilter(service, filename) : null;

  if (data === undefined || data === null) {
    return;
  }

  const serviceDir = path.join(archiveDir, service ?? '');

  let buffer;
  let filetype;

  const dataType = typeof data;

  if (dataType === 'string') {
    buffer = isFact?.([], data) ? data : scrub(data);
    filetype = '.txt';
  } else {
    // What a free-text field holds is free text all the way down
    const freeText = new WeakSet();
    // The keys that lead to each object, for the facts named by them
    const paths = new WeakMap();

    // Scrubbed value by value rather than as text, so keys, numbers and the
    // structure are left alone and doctor can still read the sample back. In
    // a sample whose fields are facts doctor decides on, only its free text.
    buffer = JSON.stringify(
      data,
      function scrubFreeText(key, value) {
        const holderPath = isFact ? paths.get(this) : undefined;
        // The sample itself is reached through the key of an outer wrapper
        const valuePath = holderPath ? [...holderPath, key] : [];

        const isFreeText = freeTextFields === null
          || freeText.has(this)
          || freeTextFields.includes(key);

        if (typeof value === 'object' && value !== null) {
          if (isFact) {
            paths.set(value, valuePath);
          }

          if (isFreeText) {
            freeText.add(value);
          }

          return value;
        }

        return typeof value === 'string' && isFreeText && !isFact?.(valuePath, value)
          ? scrub(value)
          : value;
      },
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
 * @param {*} error
 * @return {string}
 */
function describeError(error) {
  return error?.message ?? String(error);
}

/**
 * @param {string} archiveDir
 * @param {*} removalError
 * @return {string} what the operator is told when the staged samples are
 *   left behind: they are under a name no later run looks for
 */
function describeStagedSamplesLeft(archiveDir, removalError) {
  return `the staged samples could not be removed (${describeError(removalError)}): remove ${archiveDir} yourself`;
}

/**
 * @param {*} error - what stopped the archive, which comes first
 * @param {string} archiveDir
 * @param {*} removalError
 * @return {Error} the error, also saying where the staged samples were left
 */
function withStagedSamplesLeft(error, archiveDir, removalError) {
  const note = describeStagedSamplesLeft(archiveDir, removalError);
  const sentence = `${note[0].toUpperCase()}${note.slice(1)}`;

  if (error instanceof Error) {
    try {
      // eslint-disable-next-line no-param-reassign
      error.message = `${error.message}\n${sentence}`;

      return error;
    } catch {
      // An error that cannot be changed is reported inside a new one
    }
  }

  return Object.assign(
    new Error(`${describeError(error)}\n${sentence}`, { cause: error }),
    { code: error?.code },
  );
}

/**
 * @param {Samples} samples
 * @param {string} folderPath
 * @param {Config} config - the config the samples were collected from, not obfuscated
 * @return {Promise<string>} - path of the created archive
 */
export default async function archiveSamples(samples, folderPath, config) {
  if (!config) {
    throw new Error('The config the samples were collected from is needed to remove its secrets');
  }

  const scrub = createSecretScrubber(config);

  // Windows does not allow ':' in file names. The exact collection time is
  // kept in the archive's date sample, so the name only has to be readable.
  const timestamp = samples.date.toISOString().replace(/:/g, '-');
  const archiveName = `dashmate-report-${timestamp}`;
  const archiveFilePath = path.join(folderPath, `${archiveName}.tar.gz`);

  // Staged in a fresh directory only the operator can read, and removed once
  // packed: the archive is the one copy meant to leave the machine, and
  // nothing that was in the temp directory before is packed along with it.
  const archiveDir = fs.mkdtempSync(path.join(os.tmpdir(), `${archiveName}-`));

  try {
    // What dashmate writes about itself rather than collects is left out of
    // the scrub. Doctor checks its structure when it reads a report back - the
    // config copy against the config schema, the date as a date - and the
    // scrub cannot tell a secret from an equal value that has to stay, such
    // as a password spelled like the environment name. The config copy has
    // its secrets hidden field by field instead.
    const keep = (text) => text;

    writeSampleFile(archiveDir, null, 'date', samples.date.toISOString(), keep);
    writeSampleFile(archiveDir, null, 'systemInfo', samples.getSystemInfo(), scrub);
    writeSampleFile(archiveDir, null, 'dockerError', samples.getStringifiedDockerError(), scrub);
    writeSampleFile(archiveDir, null, 'dashmateConfig', samples.getDashmateConfig(), keep);
    writeSampleFile(archiveDir, null, 'dashmateVersion', samples.getDashmateVersion(), keep);

    for (const [serviceName, service] of Object.entries(samples.getServices())) {
      for (const [key, data] of Object.entries(service)) {
        if (data !== undefined && data !== null) {
          writeSampleFile(archiveDir, serviceName, key, data, scrub);
        }
      }
    }

    // The staging directory's own entry is left out: it would carry the
    // private mode onto whatever directory the archive is extracted into.
    // Listed before the archive file is opened, which only packing closes.
    const stagedFiles = fs.readdirSync(archiveDir);

    // Packed under a name only this run uses and renamed once complete, so a
    // failure never leaves a truncated archive under the report's name and
    // never touches a file this run did not create.
    const partialFilePath = `${archiveFilePath}.${crypto.randomBytes(4).toString('hex')}.partial`;

    // Opened before the guard below: `wx` refuses a file that already exists,
    // and a file this run did not create is not its own to remove. Created
    // readable by its owner only, since it can be left behind when renaming
    // and removing it both fail.
    const output = fs.createWriteStream(null, { fd: fs.openSync(partialFilePath, 'wx', 0o600) });

    try {
      // pipeline closes the file when packing fails, so it can be removed
      await pipeline(
        create(
          {
            cwd: archiveDir,
            gzip: true,
            // Without this every entry records the name and ids of the account
            // that ran the report, which the samples themselves have masked
            portable: true,
          },
          stagedFiles,
        ),
        output,
      );

      fs.renameSync(partialFilePath, archiveFilePath);
    } catch (e) {
      try {
        fs.rmSync(partialFilePath, { force: true });
      } catch {
        // The packing or rename error is the one to report
      }

      throw e;
    }
  } catch (e) {
    try {
      fs.rmSync(archiveDir, { recursive: true, force: true });
    } catch (removalError) {
      throw withStagedSamplesLeft(e, archiveDir, removalError);
    }

    throw e;
  }

  try {
    fs.rmSync(archiveDir, { recursive: true, force: true });
  } catch (removalError) {
    throw new Error(
      `The report was saved to ${archiveFilePath}, but ${describeStagedSamplesLeft(archiveDir, removalError)}`,
      { cause: removalError },
    );
  }

  return archiveFilePath;
}
