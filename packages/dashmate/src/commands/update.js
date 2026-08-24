import { Flags } from '@oclif/core';
import chalk from 'chalk';
import { Listr } from 'listr2';

import { NETWORK_MAINNET, NETWORK_TESTNET, OUTPUT_FORMATS } from '../constants.js';
import ConfigBaseCommand from '../oclif/command/ConfigBaseCommand.js';
import MuteOneLineError from '../oclif/errors/MuteOneLineError.js';
import printArrayOfObjects from '../printers/printArrayOfObjects.js';
import CertificateUnresolvedError from '../ssl/errors/CertificateUnresolvedError.js';
import { describeStatus } from '../ssl/checkGatewayCertificateFactory.js';
import {
  reportUnresolved as reportUnresolved_,
  writeDiagnostics,
} from '../ssl/certificateReporting.js';
import isEnvironmentFlagSet from '../util/isEnvironmentFlagSet.js';
import isInteractiveSession from '../util/isInteractiveSession.js';

/**
 * Networks whose certificate has to be publicly trusted. Local and devnet nodes
 * are self-signed by design and disposable.
 */
const GATED_NETWORKS = [NETWORK_MAINNET, NETWORK_TESTNET];

export default class UpdateCommand extends ConfigBaseCommand {
  // The certificate check can obtain a certificate and record the provider that
  // issued it, so it holds the configuration lock for its whole run.
  static mutatesConfig = true;

  static description = 'Update node software';

  static flags = {
    ...ConfigBaseCommand.flags,
    format: Flags.string({
      description: 'display output format',
      default: OUTPUT_FORMATS.PLAIN,
      options: Object.values(OUTPUT_FORMATS),
    }),
    'skip-certificate-check': Flags.boolean({
      description: 'do not act on the gateway certificate check. It still runs and still reports,'
        + ' but nothing is prompted, obtained or blocked. Also DASHMATE_SKIP_CERTIFICATE_CHECK',
      default: false,
    }),
    'non-interactive': Flags.boolean({
      description: 'never prompt. The certificate is checked and reported, nothing is obtained or'
        + ' changed. Also DASHMATE_NON_INTERACTIVE. Use CI=0 to prompt on a machine that exports CI',
      default: false,
    }),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {docker} docker
   * @param {Config} config
   * @param {updateNode} updateNode
   * @param {checkGatewayCertificate} checkGatewayCertificate
   * @param {gatewayCertificateTask} gatewayCertificateTask
   * @param {DockerCompose} dockerCompose
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    docker,
    config,
    updateNode,
    checkGatewayCertificate,
    gatewayCertificateTask,
    dockerCompose,
  ) {
    const {
      format,
      verbose: isVerbose,
    } = flags;

    const skipCertificateCheck = flags['skip-certificate-check'] === true
      || isEnvironmentFlagSet(process.env.DASHMATE_SKIP_CERTIFICATE_CHECK);

    const interactive = isInteractiveSession({ flags });

    const isGated = config.get('platform.enable') === true
      && GATED_NETWORKS.includes(config.get('network'));

    const reportUnresolved = (verdict, obtainAttemptFailed = false) => reportUnresolved_({
      config,
      verdict,
      dockerCompose,
      pull: this.pullResult ?? null,
      obtainAttemptFailed,
    });

    // A prompt that leaks past the interactivity guard neither throws nor
    // settles: the event loop simply drains and the process exits 0 with
    // nothing done. Failing closed here turns that silence into an exit code.
    process.exitCode = 1;

    // Both handlers are attached the moment the pull is created, so the minutes
    // the certificate task may spend at a prompt are not a window in which an
    // unhandled rejection can take the process down. updateNode is async and
    // calls getServiceList synchronously, and docker.pull can throw
    // synchronously inside its executor, so this promise really can reject.
    const settled = updateNode(config).then(
      (info) => ({ ok: true, info }),
      (error) => ({ ok: false, error }),
    );

    let pullReported = false;

    const reportPull = async () => {
      if (pullReported) {
        return;
      }
      pullReported = true;

      const result = await settled;

      if (!result.ok) {
        // Nothing was fetched at all - not a per-image failure, which resolves
        // as an error row and has always exited 0. Retained so it can be
        // raised once the certificate has had its say: returning quietly here
        // hands `update && start` a node whose images were never downloaded,
        // with no exit code for the caller to catch.
        this.pullResult = { ok: false, failed: 0, total: 0 };
        this.pullError = result.error;

        return;
      }

      this.pullResult = {
        ok: true,
        failed: result.info.filter(({ updated }) => updated === 'error').length,
        total: result.info.length,
      };

      const colors = {
        updated: chalk.yellow,
        'up to date': chalk.green,
        error: chalk.red,
      };

      printArrayOfObjects(result.info.map(({
        name, title, updated, image,
      }) => (format === OUTPUT_FORMATS.PLAIN
        ? { Service: title, Image: image, Updated: colors[updated](updated) }
        : {
          name, title, updated, image,
        })), format);
    };

    const tasks = new Listr(
      [
        {
          title: 'Gateway certificate',
          enabled: () => isGated,
          task: gatewayCertificateTask(config, { interactive, skipCertificateCheck }),
        },
        {
          title: 'Update node software',
          task: () => reportPull(),
        },
      ],
      {
        // The certificate task signals an unresolved certificate by throwing,
        // because throwing is the only way to render a listr2 task as failed.
        // Without this the throw would skip the pull report entirely, hiding
        // the table - including any image that failed to download.
        exitOnError: false,
        // Interactivity beats --verbose: the verbose renderer manages no prompt
        // area, and -v is exactly what an operator adds when the check has just
        // failed.
        renderer: (format === OUTPUT_FORMATS.JSON && 'silent')
          || (interactive && 'default')
          || (isVerbose && 'verbose')
          || 'default',
        rendererOptions: {
          showTimer: isVerbose,
          clearOutput: false,
          collapse: false,
          showSubtasks: true,
          removeEmptyLines: false,
        },
      },
    );

    const context = {};

    try {
      await tasks.run(context);
    } finally {
      // Covers what task 2 cannot: an exception from run() itself, or from the
      // reporting path. The guard makes the second call harmless, and the table
      // is rendered before any failure is reported either way.
      await reportPull();
    }

    // listr2 wraps what a task threw, so the sentinel is one level down.
    const errors = (tasks.err ?? []).map((error) => error?.error ?? error);
    const unresolved = errors.find((error) => error instanceof CertificateUnresolvedError);
    const unexpected = errors.find((error) => !(error instanceof CertificateUnresolvedError));

    // Under JSON output stdout is exactly one parseable array, so everything a
    // machine might want about the certificate goes to stderr as one line.
    if (format === OUTPUT_FORMATS.JSON && context.certificate) {
      writeDiagnostics(context.certificate, config, {
        skipped: context.certificateSkipped === true,
        pull: this.pullResult ?? null,
      });
    }

    (context.certificateWarnings ?? []).forEach((warning) => {
      process.stderr.write(`${warning}\n\n`);
    });

    if (context.certificateSkipped) {
      process.stderr.write(`Gateway certificate enforcement was skipped.`
        + ` The check still ran, and the certificate ${describeStatus(context.certificate.status)}.\n\n`);
    }

    if (context.certificateSuccess) {
      process.stderr.write(`${context.certificateSuccess}\n`);
    }

    // A lost lock, a failed reload or a programming error is a real failure and
    // must not be reduced to a certificate message. exitOnError would otherwise
    // have swallowed it.
    if (unexpected) {
      // A pull that fetched nothing renders no table and carries no message of
      // its own - it is raised further down instead. Only one error can be
      // thrown, so without saying it here the operator is told the certificate
      // failed and never learns their images never arrived. One Docker daemon
      // being down produces both at once.
      if (this.pullError) {
        process.stderr.write(`Images could not be pulled: ${this.pullError.message}\n\n`);
      }

      throw unexpected;
    }

    // Printed before either failure is raised, so an operator whose node has
    // both problems still gets the remediation for the one they can act on.
    if (unresolved) {
      await reportUnresolved(
        unresolved.getVerdict(),
        Boolean(context.certificateObtainError),
      );
    }

    // A pull that fetched nothing is what this command exists to do, so it
    // outranks the certificate: the caller has to see a non-zero exit and the
    // reason, not a muted certificate message.
    if (this.pullError) {
      throw this.pullError;
    }

    if (unresolved) {
      throw new MuteOneLineError(unresolved);
    }

    process.exitCode = 0;
  }
}
