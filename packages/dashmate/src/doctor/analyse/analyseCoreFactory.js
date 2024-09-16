import chalk from 'chalk';
import { SEVERITY } from '../Prescription.js';
import Problem from '../Problem.js';

/**
 * @return {analyseCore}
 */
export default function analyseCoreFactory() {
  /**
   * @typedef {Function} analyseCore
   * @param {Samples} samples
   * @return {Problem[]}
   */
  function analyseCore(samples) {
    const problems = [];

    // Core is synced
    const masternodeSyncStatus = samples.getServiceInfo('core', 'masternodeSyncStatus');

    if (masternodeSyncStatus?.IsSynced === false) {
      const blockchainInfo = samples.getServiceInfo('core', 'blockchainInfo');
      const verificationProgress = blockchainInfo?.verificationprogress ?? 0;

      const problem = new Problem(
        'Core is syncing blockchain data. Some node services may be temporarily unresponsive',
        chalk`Sync is ${(verificationProgress * 100).toFixed(1)}% complete. Please wait until Core is fully synced`,
        SEVERITY.MEDIUM,
      );

      problems.push(problem);
    }

    // PoSe
    if (samples?.getDashmateConfig()?.get('core.masternode.enable')) {
      const masternodeStatus = samples.getServiceInfo('core', 'masternodeStatus');

      const { description, solution, severity } = {
        WAITING_FOR_PROTX: {
          description: 'The masternode is waiting for ProTx registration confirmation',
          solution: chalk`Ensure the ProRegTx transaction has been sent and is confirmed on the network.`,
          severity: SEVERITY.HIGH,
        },
        POSE_BANNED: {
          description: 'The masternode has been banned due to failing Proof-of-Service checks.',
          solution: chalk`Submit a ProUpServTx transaction to unban your masternode and ensure
it meets all network requirements.`,
          severity: SEVERITY.HIGH,
        },
        REMOVED: {
          description: 'The masternode has been removed from the network\'s masternode list.',
          solution: chalk`Re-register the masternode with a new ProRegTx transaction.`,
          severity: SEVERITY.HIGH,
        },
        OPERATOR_KEY_CHANGED: {
          description: 'The operator key for the masternode has been changed.',
          solution: chalk`Update the masternode configuration with the new operator key
using {bold.cyanBright dashmate config set core.masternode.operatorKey <operatorKey>}.`,
          severity: SEVERITY.HIGH,
        },
        PROTX_IP_CHANGED: {
          description: 'The IP address registered in the ProTx has changed.',
          solution: chalk`Update your masternode\'s configuration with the new IP address.`,
          severity: SEVERITY.HIGH,
        },
        ERROR: {
          description: 'An unknown error has occurred with the masternode.',
          solution: chalk`Check the Core logs for detailed error information and troubleshoot accordingly.`,
          severity: SEVERITY.HIGH,
        },
        UNKNOWN: {
          description: 'The masternode status cannot be determined.',
          solution: chalk`Check the Core logs for detailed error information and troubleshoot accordingly.`,
          severity: SEVERITY.HIGH,
        },
      }[masternodeStatus?.state] || {};

      if (description) {
        const problem = new Problem(
          description,
          solution,
          severity,
        );

        problems.push(problem);
      }
    }

    return problems;
  }

  return analyseCore;
}
