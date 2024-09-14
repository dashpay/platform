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
        'Core syncs blockchain data. Some of the node services might not respond',
        chalk`${(verificationProgress * 100).toFixed(1)}% is synced. Please wait until Core will be fully synced`,
        SEVERITY.MEDIUM,
      );

      problems.push(problem);
    }

    // PoSe
    if (samples.getDashmateConfig().get('core.masternode.enable')) {
      const masternodeStatus = samples.getServiceInfo('core', 'masternodeStatus');

      const { description, solution, severity } = {
        WAITING_FOR_PROTX: {
          description: 'Problem',
          solution: chalk`Solution`,
          severity: SEVERITY.HIGH,
        },
        POSE_BANNED: {
          description: 'Problem',
          solution: chalk`Solution`,
          severity: SEVERITY.HIGH,
        },
        REMOVED: {
          description: 'Problem',
          solution: chalk`Solution`,
          severity: SEVERITY.HIGH,
        },
        OPERATOR_KEY_CHANGED: {
          description: 'Problem',
          solution: chalk`Solution`,
          severity: SEVERITY.HIGH,
        },
        PROTX_IP_CHANGED: {
          description: 'Problem',
          solution: chalk`Solution`,
          severity: SEVERITY.HIGH,
        },
        ERROR: {
          description: 'Problem',
          solution: chalk`Solution`,
          severity: SEVERITY.HIGH,
        },
        UNKNOWN: {
          description: 'Problem',
          solution: chalk`Solution`,
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
