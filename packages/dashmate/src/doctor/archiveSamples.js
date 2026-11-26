import os from 'os';
import path from 'path';
import fs from 'fs';
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

  try {
    await create(
      {
        cwd: archiveDir,
        gzip: true,
        file: archiveFilePath,
      },
      ['.'],
    );
  } catch (e) {
    // tar has already written part of the archive by the time it fails, and a
    // truncated archive would be mistaken for a report
    fs.rmSync(archiveFilePath, { force: true });

    throw e;
  }

  return archiveFilePath;
}
