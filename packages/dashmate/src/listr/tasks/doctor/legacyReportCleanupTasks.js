import {
  findLegacyReportStagingDirs,
  releaseLegacyReportStagingDirs,
  removeLegacyReportStagingDirs,
} from '../../../doctor/removeLegacyReportStagingDirs.js';

/**
 * @param {{dir: string, error: Error, restricted: boolean}} entry
 * @return {string}
 */
function describeLeft({ dir, error, restricted }) {
  return `Left ${dir}, ${restricted ? 'closed to other users' : 'could not close it to other users'}: ${error.message}`;
}

/**
 * The report command's tasks for the files earlier versions left in the temp
 * directory while creating reports.
 *
 * What they left looks the same as a report an operator unpacked there and
 * kept, so the directories found are listed and only removed once the
 * operator agrees. Anything that does not hold exactly what an earlier
 * version staged is left in place and listed with why, and so is what a
 * removal that did not finish left behind.
 *
 * @param {string} tempDir
 * @return {Object[]} Listr task definitions
 */
export default function createLegacyReportCleanupTasks(tempDir) {
  return [
    {
      title: 'Look for files left by earlier reports',
      task: async (ctx, task) => {
        const {
          found, left, recovery, scanFailure,
        } = findLegacyReportStagingDirs(tempDir);

        ctx.legacyReportDirs = found;

        const lines = [
          ...(scanFailure
            ? [`Could not look in ${scanFailure.dir} for files left by earlier reports: ${scanFailure.error.message}`]
            : []),
          ...recovery.map(({ dir }) => `Found ${dir}, left by a removal that did not finish: check what it holds and remove it yourself`),
          ...left.map(describeLeft),
          ...found.map(({ dir }) => `Found ${dir}`),
        ];

        if (lines.length === 0) {
          task.skip('Nothing found');

          return;
        }

        // eslint-disable-next-line no-param-reassign
        task.output = lines.join('\n');
      },
      options: {
        persistentOutput: true,
      },
    },
    {
      title: 'Remove files left by earlier reports',
      enabled: (ctx) => ctx.legacyReportDirs?.length > 0,
      task: async (ctx, task) => {
        let agreed;

        try {
          agreed = await task.prompt({
            type: 'toggle',
            name: 'confirm',
            header: `  Earlier dashmate versions left these directories behind while creating reports:

${ctx.legacyReportDirs.map(({ dir }) => `  - ${dir}`).join('\n')}

  They hold nothing but the files those versions wrote. A report you unpacked
  there yourself looks the same, so they are only removed if you agree.\n`,
            message: 'Remove them?',
            enabled: 'Yes',
            disabled: 'No',
          });
        } catch (error) {
          releaseLegacyReportStagingDirs(ctx.legacyReportDirs);

          throw error;
        }

        if (!agreed) {
          releaseLegacyReportStagingDirs(ctx.legacyReportDirs);

          task.skip('Left in place');

          return;
        }

        const { removed, failed } = removeLegacyReportStagingDirs(ctx.legacyReportDirs);

        // eslint-disable-next-line no-param-reassign
        task.output = [
          ...removed.map((dir) => `Removed ${dir}`),
          ...failed.map(describeLeft),
        ].join('\n');
      },
      options: {
        persistentOutput: true,
      },
    },
  ];
}
