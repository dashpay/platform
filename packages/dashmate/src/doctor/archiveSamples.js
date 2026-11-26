import crypto from 'crypto';
import os from 'os';
import path from 'path';
import fs from 'fs';
import { pipeline } from 'stream/promises';
import { create } from 'tar';

function writeSampleFile(archiveDir, service, filename, data) {
  if (data === undefined || data === null) {
    return;
  }

  const serviceDir = path.join(archiveDir, service ?? '');

  let buffer;
  let filetype;

  const dataType = typeof data;

  if (dataType === 'string') {
    buffer = data;
    filetype = '.txt';
  } else {
    buffer = JSON.stringify(data, null, 2);
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
 * @return {Promise<string>} - path of the created archive
 */
export default async function archiveSamples(samples, folderPath) {
  const tempDir = os.tmpdir();
  // Windows does not allow ':' in file names. The exact collection time is
  // kept in the archive's date sample, so the name only has to be readable.
  const timestamp = samples.date.toISOString().replace(/:/g, '-');
  const archiveName = `dashmate-report-${timestamp}`;
  const archiveDir = path.join(tempDir, archiveName);
  const archiveFilePath = path.join(folderPath, `${archiveName}.tar.gz`);

  writeSampleFile(archiveDir, null, 'date', samples.date.toISOString());
  writeSampleFile(archiveDir, null, 'systemInfo', samples.getSystemInfo());
  writeSampleFile(archiveDir, null, 'dockerError', samples.getStringifiedDockerError());
  writeSampleFile(archiveDir, null, 'dashmateConfig', samples.getDashmateConfig());
  writeSampleFile(archiveDir, null, 'dashmateVersion', samples.getDashmateVersion());

  for (const [serviceName, service] of Object.entries(samples.getServices())) {
    for (const [key, data] of Object.entries(service)) {
      if (data !== undefined && data !== null) {
        writeSampleFile(archiveDir, serviceName, key, data);
      }
    }
  }

  // Packed under a name only this run uses and renamed once complete, so a
  // failure never leaves a truncated archive under the report's name and never
  // touches a file this run did not create.
  const partialFilePath = `${archiveFilePath}.${crypto.randomBytes(4).toString('hex')}.partial`;

  try {
    // pipeline closes the file when packing fails, so it can be removed
    await pipeline(
      create({ cwd: archiveDir, gzip: true }, ['.']),
      fs.createWriteStream(partialFilePath, { flags: 'wx' }),
    );
  } catch (e) {
    try {
      fs.rmSync(partialFilePath, { force: true });
    } catch {
      // The packing error is the one to report
    }

    throw e;
  }

  fs.renameSync(partialFilePath, archiveFilePath);

  return archiveFilePath;
}
