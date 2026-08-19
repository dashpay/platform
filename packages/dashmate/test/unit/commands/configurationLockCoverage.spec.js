import SetupCommand from '../../../src/commands/setup.js';
import ResetCommand from '../../../src/commands/reset.js';
import GroupResetCommand from '../../../src/commands/group/reset.js';
import ObtainCommand from '../../../src/commands/ssl/obtain.js';
import ConfigRenderCommand from '../../../src/commands/config/render.js';
import ReindexCommand from '../../../src/commands/core/reindex.js';
import GroupReindexCommand from '../../../src/commands/group/core/reindex.js';

describe('Commands that write rendered configuration', () => {
  [
    ['setup', SetupCommand],
    ['reset', ResetCommand],
    ['group reset', GroupResetCommand],
    ['ssl obtain', ObtainCommand],
    ['config render', ConfigRenderCommand],
    ['core reindex', ReindexCommand],
    ['group core reindex', GroupReindexCommand],
  ].forEach(([name, Command]) => {
    it(`should hold the configuration lock for ${name}`, () => {
      expect(Command.mutatesConfig).to.be.true();
    });
  });
});
