import { Listr } from 'listr2';
import { Flags } from '@oclif/core';
import ServiceIsNotRunningError from '../../docker/errors/ServiceIsNotRunningError.js';
import ConfigBaseCommand from '../../oclif/command/ConfigBaseCommand.js';
import isInteractiveSession from '../../util/isInteractiveSession.js';
import renderGatewayRestartCommand from '../../ssl/renderGatewayRestartCommand.js';
import renderConfigFlag from '../../util/renderConfigFlag.js';
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
   * @param {HomeDir} homeDir
   * @param {waitForGatewayToServeBundle} waitForGatewayToServeBundle
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    config,
    obtainZeroSSLCertificateTask,
    obtainLetsEncryptCertificateTask,
    configFileRepository,
    configFile,
    dockerCompose,
    homeDir,
    waitForGatewayToServeBundle,
  ) {
    const {
      verbose: isVerbose,
      'no-retry': noRetry,
      'expiration-days': expirationDaysFlag,
      force,
      provider: providerFlag,
    } = flags;

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

    // Whether the obtain may ask a question is decided here rather than inside
    // the shared task, so a caller that never opts in - the helper's
    // unattended renewal - cannot enable prompting by omission.
    const interactive = isInteractiveSession({ flags });

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
          //
          // A signal is sufficient and nothing here needs to restart the
          // container. PID 1 in the gateway container is Envoy's hot-restarter,
          // not Envoy: its SIGHUP handler forks and re-execs Envoy with an
          // incremented restart epoch against the same envoy.yaml. The new
          // process parses that file from scratch and opens the certificate by
          // name, so both a renewed certificate and a changed listener
          // structure take effect while the old process drains. A container
          // restart would achieve the same thing and cost an outage.
          title: 'Reload gateway',
          task: async (ctx, listrTask) => {
            // A bundle the obtain wrote as a new file is not the file the
            // running gateway has mounted, so a signal would load the
            // previous certificate again. The command is printed after the
            // task list.
            if (ctx.isGatewayRestartRequired) {
              listrTask.skip('Not reloaded: the certificate bundle was written as a new file');

              return;
            }

            try {
              await dockerCompose.execCommand(config, 'gateway', 'kill -SIGHUP 1');
            } catch (e) {
              if (!(e instanceof ServiceIsNotRunningError)) {
                throw e;
              }

              listrTask.skip('Gateway is not running');

              return;
            }

            // The signal reloads the file the gateway has mounted, which need
            // not be the bundle on disk: one an earlier run wrote as a new
            // file stays mounted until the gateway restarts, and a later run
            // that finds it installed writes nothing. So what the gateway
            // serves decides, not what this run wrote.
            const served = await waitForGatewayToServeBundle(
              config,
              homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl', 'bundle.crt'),
            );

            if (served.matchesOnDisk === true) {
              return;
            }

            ctx.servedGatewayCertificate = served;

            listrTask.skip(served.matchesOnDisk === false
              ? 'Not applied: the gateway still serves a different certificate'
              : 'Signalled, but the certificate the gateway serves could not be checked');
          },
        },
      ],
      {
        // Interactivity beats --verbose: the verbose renderer manages no prompt
        // area, and logs every redraw of the retry prompt as a line of its own.
        renderer: (interactive && 'default') || (isVerbose && 'verbose') || 'default',
        rendererOptions: {
          showTimer: isVerbose,
          clearOutput: false,
          collapse: false,
          showSubtasks: true,
          removeEmptyLines: false,
        },
      },
    );

    const context = {
      noRetry,
      force,
      expirationDays,
      interactive,
    };

    try {
      await tasks.run(context);
    } catch (e) {
      throw new MuteOneLineError(e);
    }

    // On a line of its own: the task list's renderer cuts a message off at
    // the terminal width.
    if (context.isGatewayRestartRequired) {
      // eslint-disable-next-line no-console
      console.log('The certificate bundle was written as a new file, which a running gateway does'
        + ' not see.\nIf the gateway is running, restart it to serve the new certificate:\n'
        + `  ${renderGatewayRestartCommand(config)}`);
    } else if (context.servedGatewayCertificate?.matchesOnDisk === false) {
      // eslint-disable-next-line no-console
      console.log('After the reload the gateway still serves a different certificate from the one'
        + ' in bundle.crt.\nIt keeps the file it mounted when it started, and a bundle written as a'
        + ' new file since then is not that file.\nRestart the gateway to serve the new'
        + ` certificate:\n  ${renderGatewayRestartCommand(config)}`);
    } else if (context.servedGatewayCertificate) {
      // A bundle that cannot be read or parsed gives no certificate to
      // compare, just like a missing one.
      const { reason } = context.servedGatewayCertificate;
      const problem = reason
        ? `dashmate could not check which certificate it serves (${reason})`
        : 'the certificate it serves could not be compared with bundle.crt, which dashmate'
          + ' could not read as a certificate';

      // eslint-disable-next-line no-console
      console.log(`The gateway was signalled to load the certificate, but ${problem}.\n`
        + `Check it with:\n  dashmate doctor ${renderConfigFlag(config.getName())}`);
    }
  }
}
