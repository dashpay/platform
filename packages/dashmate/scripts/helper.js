import dotenv from 'dotenv';
import { asValue } from 'awilix';
import graceful from 'node-graceful';
import createDIContainer from '../src/createDIContainer.js';

// The ephemeral container each SSL provider binds to port 80 during issuance.
// One can be left orphaned if a previous helper run crashed mid-renewal.
// Keyed by provider so we only ever touch the container for the active provider
// (these are live container names, not orphan-only markers).
const PROVIDER_EPHEMERAL_CONTAINER = {
  zerossl: 'dashmate-zerossl-validation',
  letsencrypt: 'dashmate-letsencrypt-lego',
};

/**
 * Force-remove the active provider's ephemeral SSL container if it was left
 * orphaned by a previous failed/killed renewal. Scoped to the configured
 * provider and run just before scheduling renewal so it cannot interfere with
 * an unrelated provider's flow.
 *
 * @param {Docker} docker
 * @param {string} provider
 * @return {Promise<void>}
 */
async function removeOrphanedSslContainer(docker, provider) {
  const name = PROVIDER_EPHEMERAL_CONTAINER[provider];

  if (!name) {
    return;
  }

  try {
    await docker.getContainer(name).remove({ force: true });

    // eslint-disable-next-line no-console
    console.log(`Removed orphaned container: ${name}`);
  } catch (e) {
    // 404 means container doesn't exist — that's the normal case
    if (e.statusCode !== 404) {
      // eslint-disable-next-line no-console
      console.error(`Failed to remove orphaned container ${name}: ${e.message}`);
    }
  }
}

(async function main() {
  // Read environment variables from .env file
  dotenv.config();

  const args = process.argv.slice(2);

  if (args.length !== 1) {
    throw new Error('please specify config name: "yarn workspace dashmate helper testnet"');
  }

  const [configName] = args;

  // eslint-disable-next-line no-console
  console.info('Starting dashmate helper');

  const container = await createDIContainer(process.env);

  // Set up graceful shutdown to clean up any containers started during
  // SSL certificate renewal (e.g. the ZeroSSL verification server on port 80)
  const stopAllContainers = container.resolve('stopAllContainers');
  const startedContainers = container.resolve('startedContainers');

  graceful.exitOnDouble = false;
  graceful.on('exit', async () => {
    // eslint-disable-next-line no-console
    console.log('Shutting down dashmate helper, cleaning up containers...');

    try {
      await stopAllContainers(
        startedContainers.getContainers(),
        { remove: true },
      );
    } catch (e) {
      // Never let a cleanup failure escape as an unhandled rejection during
      // shutdown — that would abort the handler and could leave the port-80
      // verification container alive, the exact condition we clean up for.
      // eslint-disable-next-line no-console
      console.error(`Failed to clean up containers on shutdown: ${e.message}`);
    }
  });

  const docker = container.resolve('docker');

  // Load configs
  /**
   * @type {ConfigFileJsonRepository}
   */
  const configFileRepository = container.resolve('configFileRepository');
  /**
   * @type {writeConfigTemplates}
   */
  const writeConfigTemplates = container.resolve('writeConfigTemplates');

  const configFile = await configFileRepository.read();

  // Persist config if it was migrated
  if (configFile.isChanged()) {
    await configFileRepository.write(configFile);

    configFile.getAllConfigs()
      .filter((config) => config.isChanged())
      .forEach(writeConfigTemplates);
  }

  const config = configFile.getConfig(configName);

  // Register config collection in the container
  container.register({
    configFile: asValue(configFile),
  });

  const provider = config.get('platform.gateway.ssl.provider');
  const isEnabled = config.get('platform.gateway.ssl.enabled');

  if (isEnabled && provider === 'zerossl') {
    // Clear any container left orphaned (bound to port 80) by a previous
    // failed/killed renewal before scheduling the next one
    await removeOrphanedSslContainer(docker, provider);

    const scheduleRenewZeroSslCertificate = container.resolve('scheduleRenewZeroSslCertificate');
    await scheduleRenewZeroSslCertificate(config);
  } else if (isEnabled && provider === 'letsencrypt') {
    await removeOrphanedSslContainer(docker, provider);

    const scheduleRenewLetsEncryptCertificate = container.resolve('scheduleRenewLetsEncryptCertificate');
    await scheduleRenewLetsEncryptCertificate(config);
  } else {
    // prevent infinite restarts
    setInterval(() => {
    }, 60 * 1000);
  }

  if (config.get('dashmate.helper.api.enable')) {
    const createHttpApiServer = container.resolve('createHttpApiServer');

    const httpAPIServer = createHttpApiServer();

    const port = config.get('dashmate.helper.api.port');

    httpAPIServer
      // eslint-disable-next-line no-console
      .listen(port, () => console.log(`Dashmate JSON-RPC API started on port ${port}`));
  }
}());
