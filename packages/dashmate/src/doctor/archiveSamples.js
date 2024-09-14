import os from 'os';
import path from 'path';
import fs from 'fs';
import { create } from 'tar';

function writeSampleFile(archiveDir, service, filename, data) {
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
 */
export default async function archiveSamples(samples, folderPath) {
  const tempDir = os.tmpdir();
  const archiveName = `dashmate-report-${this.date.toISOString()}`;
  const archiveDir = path.join(tempDir, archiveName);

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

  await create(
    {
      cwd: archiveDir,
      gzip: true,
      file: path.join(folderPath, `${archiveName}.tar.gz`),
    },
    ['.'],
  );
}
