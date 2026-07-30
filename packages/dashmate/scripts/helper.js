import dotenv from 'dotenv';
import { asValue } from 'awilix';
import graceful from 'node-graceful';
import createDIContainer from '../src/createDIContainer.js';

// The ephemeral containers SSL providers bind to port 80 during issuance.
// Either can be left orphaned if a previous helper run crashed mid-renewal.
const EPHEMERAL_SSL_CONTAINERS = [
  'dashmate-zerossl-validation',
  'dashmate-letsencrypt-lego',
];

/**
 * Force-remove any ephemeral SSL container left orphaned (bound to port 80) by a
 * previous failed/killed renewal. Cleans the containers for BOTH providers — not
 * just the configured one — so switching provider (e.g. zerossl -> letsencrypt)
 * cannot leave the other provider's orphan holding port 80 and blocking the next
 * renewal. Only called when SSL is enabled, before scheduling begins.
 *
 * @param {Docker} docker
 * @return {Promise<void>}
 */
async function removeOrphanedSslContainers(docker) {
  await Promise.all(EPHEMERAL_SSL_CONTAINERS.map(async (name) => {
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
  }));
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

  // Persist config if it was migrated. Nothing else here ever saves this copy -
  // it lives for the life of the process, so it goes stale, and the renewal
  // re-applies its result onto current state instead.
  if (configFile.isChanged()) {
    const changedConfigs = configFile.getAllConfigs()
      .filter((config) => config.isChanged());

    await configFileRepository.write(configFile);

    changedConfigs.forEach(writeConfigTemplates);
  }

  const config = configFile.getConfig(configName);

  // Register config collection in the container
  container.register({
    configFile: asValue(configFile),
  });

  const provider = config.get('platform.gateway.ssl.provider');
  const isEnabled = config.get('platform.gateway.ssl.enabled');

  if (isEnabled && (provider === 'zerossl' || provider === 'letsencrypt')) {
    // Clear any ephemeral SSL container left orphaned (bound to port 80) by a
    // previous failed/killed renewal before scheduling the next one. Both
    // providers are cleaned regardless of which is configured, so a provider
    // switch cannot leave the other's orphan holding port 80.
    await removeOrphanedSslContainers(docker);
  }

  if (isEnabled && provider === 'zerossl') {
    const scheduleRenewZeroSslCertificate = container.resolve('scheduleRenewZeroSslCertificate');
    await scheduleRenewZeroSslCertificate(config);
  } else if (isEnabled && provider === 'letsencrypt') {
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
