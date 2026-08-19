import { Listr } from 'listr2';
import { Flags } from '@oclif/core';
import ServiceIsNotRunningError from '../../docker/errors/ServiceIsNotRunningError.js';
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
          // Envoy reads the certificate files once at startup, so a gateway that
          // is already up keeps serving the previous certificate until it is
          // told to reload. Without this the command reports success while
          // nothing changes on the wire.
          //
          // This runs whenever the gateway is up, including when the obtain
          // wrote no new files: providers install the pair by different routes,
          // and nothing on disk reveals which certificate Envoy currently
          // holds, so an obtain that skipped the write is also how an operator
          // retries a reload that failed earlier.
          //
          // The gateway is signalled without asking first whether it is running.
          // execCommand makes that check itself, and asking separately leaves a
          // gap in which the answer can change - the certificate has already
          // been obtained by then, so failing there would report the whole
          // command as failed and send the operator back to a provider that may
          // have nothing left to issue.
          title: 'Reload gateway',
          task: async (ctx, listrTask) => {
            try {
              await dockerCompose.execCommand(config, 'gateway', 'kill -SIGHUP 1');
            } catch (e) {
              if (!(e instanceof ServiceIsNotRunningError)) {
                throw e;
              }

              listrTask.skip('Gateway is not running');
            }
          },
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
