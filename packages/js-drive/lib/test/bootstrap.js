const path = require('path');
const dotenvSafe = require('dotenv-safe');
const dotenvExpand = require('dotenv-expand');
const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const DashApiOptions = require('@dashevo/js-evo-services-ctl/lib/services/driveApi/DriveApiOptions');
const DashSyncOptions = require('@dashevo/js-evo-services-ctl/lib/services/driveSync/DriveSyncOptions');


use(sinonChai);
use(chaiAsPromised);
use(dirtyChai);

process.env.NODE_ENV = 'test';

const dotenvConfig = dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});
dotenvExpand(dotenvConfig);

const rootPath = process.cwd();
const options = {
  cacheNodeModules: true,
  localAppPath: rootPath,
  container: {
    volumes: [
      `${rootPath}/lib:/usr/src/app/lib`,
      `${rootPath}/scripts:/usr/src/app/scripts`,
      `${rootPath}/.env:/usr/src/app/.env`,
      `${rootPath}/.env.example:/usr/src/app/.env.example`,
    ],
  },
};
DashApiOptions.setDefaultCustomOptions(options);
DashSyncOptions.setDefaultCustomOptions(options);

beforeEach(function beforeEach() {
  if (!this.sinon) {
    this.sinon = sinon.createSandbox();
  } else {
    this.sinon.restore();
  }
});

global.expect = expect;
