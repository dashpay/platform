import { Listr } from 'listr2';
import { Flags } from '@oclif/core';
import ConfigBaseCommand from '../../oclif/command/ConfigBaseCommand.js';
import MuteOneLineError from '../../oclif/errors/MuteOneLineError.js';
import Certificate from '../../ssl/zerossl/Certificate.js';
import LegoCertificate from '../../ssl/letsencrypt/LegoCertificate.js';
import { SSL_PROVIDERS } from '../../constants.js';

export default class ObtainCommand extends ConfigBaseCommand {
  // Reconfigures the node: changes configuration repeatedly while doing long,
  // partly irreversible work, so it holds the config lock for its whole run.
  static mutatesConfig = true;

  static description = `Obtain SSL certificate

Create a new SSL certificate or download an already existing one using ZeroSSL or Let's Encrypt as provider
Certificate will be renewed if it is about to expire (see 'expiration-days' flag)
`;

  static flags = {
    ...ConfigBaseCommand.flags,
    verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
    'no-retry': Flags.boolean({ description: 'do not retry on IP verification failure', default: false }),
    force: Flags.boolean({ description: 'renew even if certificate is valid', default: false }),
    'expiration-days': Flags.integer({
      description: 'renew even if expiration period is less than'
        + ' specified number of days',
    }),
    provider: Flags.string({
      description: 'SSL provider to use (defaults to configured provider)',
      options: [SSL_PROVIDERS.ZEROSSL, SSL_PROVIDERS.LETSENCRYPT],
    }),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @param {obtainZeroSSLCertificateTask} obtainZeroSSLCertificateTask
   * @param {obtainLetsEncryptCertificateTask} obtainLetsEncryptCertificateTask
   * @param {ConfigFileJsonRepository} configFileRepository
   * @param {ConfigFile} configFile
   * @param {DockerCompose} dockerCompose
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
      'no-retry': noRetry,
      'expiration-days': expirationDaysFlag,
      force,
      provider: providerFlag,
    },
    config,
    obtainZeroSSLCertificateTask,
    obtainLetsEncryptCertificateTask,
    configFileRepository,
    configFile,
    dockerCompose,
  ) {
    const provider = providerFlag || config.get('platform.gateway.ssl.provider');

    let task;
    let taskTitle;
    let expirationDays;

    if (provider === SSL_PROVIDERS.LETSENCRYPT) {
      task = obtainLetsEncryptCertificateTask;
      taskTitle = "Obtain Let's Encrypt certificate";
      expirationDays = expirationDaysFlag ?? LegoCertificate.EXPIRATION_LIMIT_DAYS;
    } else if (provider === SSL_PROVIDERS.ZEROSSL) {
      task = obtainZeroSSLCertificateTask;
      taskTitle = 'Obtain ZeroSSL certificate';
      expirationDays = expirationDaysFlag ?? Certificate.EXPIRATION_LIMIT_DAYS;
    } else {
      throw new Error(`SSL provider '${provider}' does not support certificate obtainment via this command. `
        + `Supported providers: ${SSL_PROVIDERS.ZEROSSL}, ${SSL_PROVIDERS.LETSENCRYPT}`);
    }

    const taskOptions = provider === SSL_PROVIDERS.ZEROSSL
      ? {
        onCertificateCreated: () => configFileRepository.write(configFile),
      }
      : {};

    const tasks = new Listr(
      [
        {
          title: taskTitle,
          task: () => task(config, taskOptions),
        },
        {
          // Writing the certificate files does not change what the gateway serves. Without
          // this it keeps presenting the previous certificate until the node is restarted,
          // so the command reports success while nothing changes for clients.
          title: 'Reload gateway',
          enabled: () => config.get('platform.enable'),
          skip: async () => !(await dockerCompose.isServiceRunning(config, 'gateway'))
            && 'Gateway is not running, the certificate will be used when it starts',
          task: async () => dockerCompose.execCommand(config, 'gateway', 'kill -SIGHUP 1'),
        },
      ],
      {
        renderer: isVerbose ? 'verbose' : 'default',
        rendererOptions: {
          showTimer: isVerbose,
          clearOutput: false,
          collapse: false,
          showSubtasks: true,
          removeEmptyLines: false,
        },
      },
    );

    try {
      await tasks.run({
        noRetry,
        force,
        expirationDays,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}
