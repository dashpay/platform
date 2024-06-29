import {Command, HelpBase} from '@oclif/core';

export default class CustomHelp extends HelpBase {
  showHelp(args) {
    console.log('This will be displayed in multi-command CLIs');
  }

  showCommandHelp(command, topics) {
    console.log('This will be displayed in single-command CLIs');
  }
}
