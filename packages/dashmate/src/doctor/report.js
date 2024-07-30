import os from 'os';
import path from 'path';
import fs from 'fs';
import { create } from 'tar';
import generateRandomString from '../util/generateRandomString.js';

export default class Report {
  id;

  osInfo = {};

  services = {};

  constructor() {
    this.id = generateRandomString(8);
  }

  setOSInfo(osInfo) {
    this.osInfo = osInfo;
  }

  setData(service, key, data) {
    this.services[service] = {
      ...(this.services[service] ?? {}),
      [key]: data,
    };
  }

  #writeReportFile(service, filename, data) {
    const tempDir = os.tmpdir();
    const reportDir = path.join(tempDir, `dashmate-report-${this.id}`);
    const serviceDir = path.join(reportDir, service ?? '');

    let buffer;
    let filetype;

    const dataType = typeof data;

    if (dataType === 'string') {
      buffer = data;
      filetype = '.txt';
    } else if (dataType === 'object') {
      buffer = JSON.stringify(data);
      filetype = '.json';
    } else {
      throw new Error('Unknown data type');
    }

    if (!fs.existsSync(serviceDir)) {
      fs.mkdirSync(serviceDir);
    }

    fs.writeFileSync(path.join(serviceDir, `${filename}${filetype}`), buffer, 'utf8');
  }

  async archive(folderPath) {
    this.#writeReportFile(null, 'osInfo', this.osInfo);

    const tempDir = os.tmpdir();
    const reportDir = path.join(tempDir, `dashmate-report-${this.id}`);

    for (const service of Object.keys(this.services)) {
      for (const dataKey of Object.keys(this.services[service])) {
        let data = this.services[service][dataKey];

        if (data) {
          if (dataKey === 'dockerInfo') {
            const {
              exitCode, status, stdOut, stdErr,
            } = data;

            this.#writeReportFile(service, 'stdOut', stdOut);
            this.#writeReportFile(service, 'stdErr', stdErr);

            data = { exitCode, status };
          }

          this.#writeReportFile(service, dataKey, data);
        }
      }
    }

    await create(
      {
        cwd: reportDir,
        gzip: false,
        file: path.join(folderPath, `dashmate-report-${this.id}.tar`),
      },
      ['.'],
    );
  }
}
