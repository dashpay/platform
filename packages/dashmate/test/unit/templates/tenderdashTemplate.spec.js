import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import renderTemplateFactory from '../../../src/templates/renderTemplateFactory.js';

describe('Tenderdash config template', () => {
  it('should omit the first and last bootstrap peers when they match the local node ID', () => {
    const config = getBaseConfigFactory()();
    const self = { id: 'a'.repeat(40), host: '192.0.2.1', port: 26656 };
    const first = { id: 'b'.repeat(40), host: '192.0.2.2', port: 26657 };
    const second = { id: 'c'.repeat(40), host: '192.0.2.3', port: 26658 };
    config.set('platform.drive.tenderdash.node.id', self.id);
    config.set('platform.drive.tenderdash.p2p.seeds', [self, first, second, self]);
    config.set('platform.drive.tenderdash.p2p.persistentPeers', [second]);

    const template = new URL('../../../templates/platform/drive/tenderdash/config.toml.dot', import.meta.url);
    const lines = renderTemplateFactory()(template, config.options).split('\n');
    expect(lines).to.include(`bootstrap-peers = "${first.id}@192.0.2.2:26657,${second.id}@192.0.2.3:26658"`);
    expect(lines).to.include(`persistent-peers = "${second.id}@192.0.2.3:26658"`);
  });
});
