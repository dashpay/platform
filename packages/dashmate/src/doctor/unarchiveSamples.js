import os from 'os';
import path from 'path';
import fs from 'fs';
import { extract } from 'tar';
import Samples from './Samples.js';

function readSampleFile(filePath) {
  const data = fs.readFileSync(filePath, 'utf8');
  const ext = path.extname(filePath);

  if (ext === '.json') {
    return JSON.parse(data);
  }

  return data;
}

/**
 * @param {string} archiveFilePath
 * @returns {Samples}
 */
export default async function unarchiveSamples(archiveFilePath) {
  if (!fs.existsSync(archiveFilePath)) {
    throw new Error(`Archive file with samples not found: ${archiveFilePath}`);
  }

  const samples = new Samples();

  const tempDir = os.tmpdir();
  const extractDir = path.join(tempDir, archiveFilePath);

  await extract({
    file: archiveFilePath,
    cwd: extractDir,
  });

  samples.setSystemInfo(readSampleFile(path.join(extractDir, 'systemInfo.json')));
  samples.setDashmateConfig(readSampleFile(path.join(extractDir, 'dashmateConfig.json')));
  samples.setDashmateVersion(readSampleFile(path.join(extractDir, 'dashmateVersion.json')));

  const servicesDir = path.join(extractDir, 'services');
  if (fs.existsSync(servicesDir)) {
    const serviceNames = fs.readdirSync(servicesDir);

    for (const serviceName of serviceNames) {
      const serviceDir = path.join(servicesDir, serviceName);

      if (!fs.statSync(serviceDir).isDirectory()) {
        continue;
      }

      const files = fs.readdirSync(serviceDir);

      for (const file of files) {
        const filePath = path.join(serviceDir, file);

        const ext = path.extname(file);
        if (ext !== '.txt' && ext !== '.json' && !fs.statSync(filePath).isDirectory()) {
          continue;
        }

        const data = readSampleFile(filePath);
        const key = path.basename(file, ext);
        samples.setServiceInfo(serviceName, key, data);
      }
    }
  }

  fs.rmSync(extractDir, { recursive: true });

  return samples;
}
