import os from 'os';
import path from 'path';
import fs from 'fs';
import { extract, Parser } from 'tar';
import Samples from './Samples.js';
import Config from '../config/Config.js';

const ALLOWED_ENTRY_TYPES = new Set([
  'File',
  'OldFile',
  'ContiguousFile',
  'Directory',
]);
const MAX_ARCHIVE_MEMBERS = 10_000;
const MAX_EXPANDED_BYTES = 256 * 1024 * 1024;

function readSampleFile(filePath) {
  const data = fs.readFileSync(filePath, 'utf8');
  const ext = path.extname(filePath);

  if (ext === '.json') {
    return JSON.parse(data);
  }

  return data;
}

function assertSafeArchiveMember(memberPath, entry, budget) {
  const normalizedPath = path.posix.normalize(memberPath);

  if (
    path.posix.isAbsolute(memberPath)
    || memberPath.includes('\\')
    || normalizedPath === '..'
    || normalizedPath.startsWith('../')
  ) {
    throw new Error('Unsafe diagnostic archive member path');
  }

  if (!ALLOWED_ENTRY_TYPES.has(entry.type)) {
    throw new Error(`Unsupported diagnostic archive entry type: ${entry.type}`);
  }

  // eslint-disable-next-line no-param-reassign
  budget.members += 1;
  // eslint-disable-next-line no-param-reassign
  budget.bytes += Number(entry.size) || 0;

  if (budget.seen.has(normalizedPath)) {
    throw new Error('Diagnostic archive contains a duplicate member');
  }
  budget.seen.add(normalizedPath);

  if (budget.members > MAX_ARCHIVE_MEMBERS || budget.bytes > MAX_EXPANDED_BYTES) {
    throw new Error('Diagnostic archive exceeds extraction budget');
  }

  return true;
}

async function validateArchive(archivePath) {
  const budget = { members: 0, bytes: 0, seen: new Set() };

  await new Promise((resolve, reject) => {
    const source = fs.createReadStream(archivePath);
    const parser = new Parser({
      file: archivePath,
      strict: true,
      onReadEntry: (entry) => {
        try {
          assertSafeArchiveMember(entry.path, entry, budget);
          entry.resume();
        } catch (error) {
          // Stop compressed input and the parser immediately. In particular, do not let tar's
          // list helper resume entries and decompress the rest of an over-budget archive.
          source.unpipe(parser);
          source.destroy();
          parser.abort(error);
        }
      },
    });

    let settled = false;
    const finish = (error) => {
      if (settled) {
        return;
      }
      settled = true;
      source.destroy();
      if (error) {
        reject(error);
      } else {
        resolve();
      }
    };

    source.once('error', finish);
    parser.once('error', finish);
    parser.once('end', () => finish());
    source.pipe(parser);
  });
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
      throw new Error(`Archive file with logged data not found: ${archiveFilePath}`);
    }

    const privateRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-doctor-'));
    fs.chmodSync(privateRoot, 0o700);

    try {
      const rootStat = fs.lstatSync(privateRoot);
      if (!rootStat.isDirectory() || rootStat.isSymbolicLink()) {
        throw new Error('Unsafe diagnostic extraction root');
      }

      const stagedArchivePath = path.join(privateRoot, 'samples.tar.gz');
      const extractDir = path.join(privateRoot, 'contents');
      fs.copyFileSync(archiveFilePath, stagedArchivePath, fs.constants.COPYFILE_EXCL);
      fs.mkdirSync(extractDir, { mode: 0o700 });

      await validateArchive(stagedArchivePath);

      await extract({
        file: stagedArchivePath,
        cwd: fs.realpathSync(extractDir),
        preservePaths: false,
        strict: true,
        unlink: true,
        filter: (memberPath, entry) => ALLOWED_ENTRY_TYPES.has(entry.type),
      });

      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.debug(`Extracted logged data to: ${extractDir}`);
      }

      const samples = new Samples();
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

        if (!fs.existsSync(serviceDir) || !fs.lstatSync(serviceDir).isDirectory()) {
          continue;
        }

        const files = fs.readdirSync(serviceDir);

        for (const file of files) {
          const filePath = path.join(serviceDir, file);
          const fileStat = fs.lstatSync(filePath);
          const ext = path.extname(file);

          if (!fileStat.isFile() || (ext !== '.txt' && ext !== '.json')) {
            continue;
          }

          const data = readSampleFile(filePath);
          const key = path.basename(file, ext);
          samples.setServiceInfo(serviceName, key, data);
        }
      }

      return samples;
    } finally {
      if (!process.env.DEBUG) {
        fs.rmSync(privateRoot, { recursive: true, force: true });
      }
    }
  }

  return unarchiveSamples;
}
