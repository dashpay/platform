import os from 'os';
import path from 'path';
import fs from 'fs';
import { extract } from 'tar';
import Samples from './Samples.js';
import Config from '../config/Config.js';

function readSampleFile(filePath) {
  const data = fs.readFileSync(filePath, 'utf8');
  const ext = path.extname(filePath);

  if (ext === '.json') {
    return JSON.parse(data);
  }

  return data;
}

/**
 * @param {getServiceList} getServiceList
 * @returns {unarchiveSamples}
 */
export default function unarchiveSamplesFactory(getServiceList) {
  /**
   * @typedef {Function} unarchiveSamples
   * @param {string} archiveFilePath
   * @returns {Promise<Samples>}
   */
  async function unarchiveSamples(archiveFilePath) {
    if (!fs.existsSync(archiveFilePath)) {
      throw new Error(`Archive file with samples not found: ${archiveFilePath}`);
    }

    const samples = new Samples();

    const tempDir = os.tmpdir();
    const archiveFileName = path.basename(archiveFilePath, '.tar.gz');
    const extractDir = path.join(tempDir, archiveFileName);
    fs.mkdirSync(extractDir, { recursive: true });

    await extract({
      file: archiveFilePath,
      cwd: extractDir,
    });

    if (process.env.DEBUG) {
      console.debug(`Extracted samples to: ${extractDir}`);
    }

    const dateFilePath = path.join(extractDir, 'date.txt');
    if (fs.existsSync(dateFilePath)) {
      samples.date = readSampleFile(dateFilePath);
    }

    const systemInfoFilePath = path.join(extractDir, 'systemInfo.json');
    if (fs.existsSync(systemInfoFilePath)) {
      samples.setSystemInfo(readSampleFile(systemInfoFilePath));
    }

    const dockerErrorFilePath = path.join(extractDir, 'dockerError.txt');
    if (fs.existsSync(dockerErrorFilePath)) {
      samples.setStringifiedDockerError(readSampleFile(dockerErrorFilePath));
    }

    const dashmateConfigFilePath = path.join(extractDir, 'dashmateConfig.json');
    if (fs.existsSync(dashmateConfigFilePath)) {
      const configProperties = readSampleFile(dashmateConfigFilePath);
      if (configProperties?.options) {
        const config = new Config(configProperties.name, configProperties.options);
        samples.setDashmateConfig(config);
      }
    }

    const dashmateVersionFilePath = path.join(extractDir, 'dashmateVersion.txt');
    if (fs.existsSync(dashmateVersionFilePath)) {
      samples.setDashmateVersion(readSampleFile(dashmateVersionFilePath));
    }

    const serviceNames = getServiceList(samples.getDashmateConfig())
      .map((service) => service.name);

    for (const serviceName of serviceNames) {
      const serviceDir = path.join(extractDir, serviceName);

      if (!fs.statSync(serviceDir)
        .isDirectory()) {
        continue;
      }

      const files = fs.readdirSync(serviceDir);

      for (const file of files) {
        const filePath = path.join(serviceDir, file);

        const ext = path.extname(file);
        if (ext !== '.txt' && ext !== '.json' && !fs.statSync(filePath)
          .isDirectory()) {
          continue;
        }

        const data = readSampleFile(filePath);
        const key = path.basename(file, ext);
        samples.setServiceInfo(serviceName, key, data);
      }
    }

    if (!process.env.DEBUG) {
      fs.rmSync(extractDir, { recursive: true });
    }

    return samples;
  }
  return unarchiveSamples;
}
