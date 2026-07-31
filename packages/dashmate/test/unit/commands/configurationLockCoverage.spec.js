import ConfigRenderCommand from '../../../src/commands/config/render.js';
import ReindexCommand from '../../../src/commands/core/reindex.js';
import GroupReindexCommand from '../../../src/commands/group/core/reindex.js';

describe('Commands that write rendered configuration', () => {
  [
    ['config render', ConfigRenderCommand],
    ['core reindex', ReindexCommand],
    ['group core reindex', GroupReindexCommand],
  ].forEach(([name, Command]) => {
    it(`should hold the configuration lock for ${name}`, () => {
      expect(Command.mutatesConfig).to.be.true();
    });
  });
});
