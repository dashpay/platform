require('dotenv-expand')(require('dotenv-safe').config());

const connect = require('connect');
const jayson = require('jayson/promise');

const ApiApp = require('../lib/app/ApiApp');
const ApiAppOptions = require('../lib/app/ApiAppOptions');

const errorHandler = require('../lib/util/errorHandler');

(async function main() {
  const apiAppOptions = new ApiAppOptions(process.env);
  const apiApp = new ApiApp(apiAppOptions);

  await apiApp.init();

  const rpc = jayson.server(apiApp.createRpcMethodsWithNames());
  const server = connect();

  server.use(apiApp.createCheckSyncStateHttpMiddleware());
  server.use(rpc.middleware());

  server.listen(
    apiAppOptions.getApiRpcPort(),
    apiAppOptions.getApiRpcHost,
  );
}()).catch(errorHandler);

