/* eslint-disable */

// noinspection NpmUsedModulesInstalled
const logUpdate = require('log-update');
// noinspection NpmUsedModulesInstalled
const chalk = require('chalk');
// noinspection NpmUsedModulesInstalled
const figures = require('figures');
// noinspection NpmUsedModulesInstalled
const indentString = require('indent-string');
// noinspection NpmUsedModulesInstalled
const cliTruncate = require('cli-truncate');
// noinspection NpmUsedModulesInstalled
const stripAnsi = require('strip-ansi');
// noinspection NpmUsedModulesInstalled
const utils = require('listr-update-renderer/lib/utils');

const renderHelper = (tasks, options, level) => {
  level = level || 0;

  let output = [];

  for (const task of tasks) {
    if (task.isEnabled()) {
      const skipped = task.isSkipped() ? ` ${chalk.dim('[skipped]')}` : '';

      output.push(indentString(` ${utils.getSymbol(task, options)} ${task.title}${skipped}`, level, '  '));

      if ((task.isCompleted() || task.isPending() || task.isSkipped() || task.hasFailed()) && utils.isDefined(task.output)) {
        let data = task.output;

        if (typeof data !== 'string' && data !== null && data !== undefined) {
          data = data.toString();
        }

        data = data.trim().split('\n').map(stripAnsi).filter(Boolean);

        if (!task.isCompleted()) {
          data = data.slice(0, 1);
        }

        if (utils.isDefined(data)) {
          data.forEach((line) => {
            const out = indentString(`${figures.arrowRight} ${line}`, level, '  ');
            output.push(`   ${chalk.gray(cliTruncate(out, process.stdout.columns - 3))}`);
          });
        }
      }

      if ((task.isPending() || task.hasFailed() || options.collapse === false) && (task.hasFailed() || options.showSubtasks !== false) && task.subtasks.length > 0) {
        output = output.concat(renderHelper(task.subtasks, options, level + 1));
      }
    }
  }

  return output.join('\n');
};

const render = (tasks, options) => {
  logUpdate(renderHelper(tasks, options));
};

class UpdateRendererWithOutput {
  constructor(tasks, options) {
    this._tasks = tasks;
    this._options = Object.assign({
      showSubtasks: true,
      collapse: true,
      clearOutput: false
    }, options);
  }

  // noinspection JSUnusedGlobalSymbols
  render() {
    if (this._id) {
      // Do not render if we are already rendering
      return;
    }

    this._id = setInterval(() => {
      render(this._tasks, this._options);
    }, 100);
  }

  // noinspection JSUnusedGlobalSymbols
  end(err) {
    if (this._id) {
      clearInterval(this._id);
      this._id = undefined;
    }

    render(this._tasks, this._options);

    if (this._options.clearOutput && err === undefined) {
      logUpdate.clear();
    } else {
      logUpdate.done();
    }
  }
}

module.exports = UpdateRendererWithOutput;
