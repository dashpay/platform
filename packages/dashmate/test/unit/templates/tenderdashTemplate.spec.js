import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import renderServiceTemplatesFactory from '../../../src/templates/renderServiceTemplatesFactory.js';
import renderTemplateFactory from '../../../src/templates/renderTemplateFactory.js';
import deriveTenderdashNodeId from '../../../src/tenderdash/deriveTenderdashNodeId.js';
import generateTenderdashNodeKey from '../../../src/tenderdash/generateTenderdashNodeKey.js';

describe('Tenderdash config template', () => {
  const render = renderServiceTemplatesFactory(renderTemplateFactory());
  const configPath = 'platform/drive/tenderdash/config.toml';

  it('should omit the first and last bootstrap peers when they match the local node ID', () => {
    const config = getBaseConfigFactory()();
    const self = { id: 'a'.repeat(40), host: '192.0.2.1', port: 26656 };
    const first = { id: 'b'.repeat(40), host: '192.0.2.2', port: 26657 };
    const second = { id: 'c'.repeat(40), host: '192.0.2.3', port: 26658 };
    config.set('platform.drive.tenderdash.node.id', self.id);
    config.set('platform.drive.tenderdash.p2p.seeds', [self, first, second, self]);
    config.set('platform.drive.tenderdash.p2p.persistentPeers', [second]);

    const lines = render(config)[configPath].split('\n');
    expect(lines).to.include(`bootstrap-peers = "${first.id}@192.0.2.2:26657,${second.id}@192.0.2.3:26658"`);
    expect(lines).to.include(`persistent-peers = "${second.id}@192.0.2.3:26658"`);
  });

  it('should omit custom bootstrap peers matching the key-derived ID in another case', () => {
    const config = getBaseConfigFactory()();
    const key = generateTenderdashNodeKey();
    const self = { id: deriveTenderdashNodeId(key).toUpperCase(), host: '192.0.2.1', port: 26656 };
    const stale = { id: 'a'.repeat(40), host: '192.0.2.2', port: 26657 };
    config.set('platform.drive.tenderdash.node', { id: stale.id, key });
    config.set('platform.drive.tenderdash.p2p.seeds', [self, stale]);

    expect(render(config)[configPath].split('\n'))
      .to.include(`bootstrap-peers = "${stale.id}@192.0.2.2:26657"`);
    expect(config.getStored('platform.drive.tenderdash.p2p.seeds')).to.deep.equal([self, stale]);
  });

  for (const seeds of [null, undefined]) {
    it(`should render an empty bootstrap list when seeds are ${seeds}`, () => {
      const config = getBaseConfigFactory()();
      const options = config.getStoredOptions();
      const { p2p } = options.platform.drive.tenderdash;
      if (seeds === undefined) {
        delete p2p.seeds;
      } else {
        p2p.seeds = seeds;
      }
      config.setOptions(options, true);

      expect(render(config)[configPath].split('\n')).to.include('bootstrap-peers = ""');
      expect(config.getStored('platform.drive.tenderdash.p2p')).to.deep.equal(p2p);
    });
  }
});
