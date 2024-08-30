import os from 'os';
import path from 'path';
import fs from 'fs';
import { create } from 'tar';

export default class Report {
  date;

  #systemInfo = {};

  #dashmateVersion = null;

  #dashmateConfig = null;

  #services = {};

  constructor() {
    this.date = new Date();
  }

  setSystemInfo(systemInfo) {
    this.#systemInfo = systemInfo;
  }

  setDashmateVersion(version) {
    this.#dashmateVersion = version;
  }

  setDashmateConfig(config) {
    this.#dashmateConfig = config;
  }

  setServiceInfo(service, key, data) {
    this.#services[service] = {
      ...(this.#services[service] ?? {}),
      [key]: data,
    };
  }

  #writeReportFile(reportDir, service, filename, data) {
    const serviceDir = path.join(reportDir, service ?? '');

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

  async archive(folderPath) {
    const tempDir = os.tmpdir();
    const reportName = `dashmate-report-${this.date.toISOString()}`;
    const reportDir = path.join(tempDir, reportName);

    this.#writeReportFile(reportDir, null, 'systemInfo', this.#systemInfo);
    this.#writeReportFile(reportDir, null, 'dashmateConfig', this.#dashmateConfig);
    this.#writeReportFile(reportDir, null, 'dashmateVersion', this.#dashmateVersion);

    for (const service of Object.keys(this.#services)) {
      for (const dataKey of Object.keys(this.#services[service])) {
        const data = this.#services[service][dataKey];

        if (data !== undefined && data !== null) {
          this.#writeReportFile(reportDir, service, dataKey, data);
        }
      }
    }

    await create(
      {
        cwd: reportDir,
        gzip: true,
        file: path.join(folderPath, `${reportName}.tar.gz`),
      },
      ['.'],
    );
  }
}
