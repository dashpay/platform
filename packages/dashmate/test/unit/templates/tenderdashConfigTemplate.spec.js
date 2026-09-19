import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import HomeDir from '../../../src/config/HomeDir.js';
import renderServiceTemplatesFactory from '../../../src/templates/renderServiceTemplatesFactory.js';
import renderTemplateFactory from '../../../src/templates/renderTemplateFactory.js';

describe('tenderdash config template', () => {
  let config;
  let renderServiceTemplates;

  beforeEach(() => {
    const getBaseConfig = getBaseConfigFactory(HomeDir.createTemp());
    config = getBaseConfig();

    const renderTemplate = renderTemplateFactory();
    renderServiceTemplates = renderServiceTemplatesFactory(renderTemplate);
  });

  const renderTenderdashConfig = () => renderServiceTemplates(config)['platform/drive/tenderdash/config.toml'];

  it('should render the statesync section from config defaults', () => {
    const toml = renderTenderdashConfig();

    expect(toml).to.include('enable = true');
    expect(toml).to.include('use-p2p = true');
    expect(toml).to.include('retries = 3');
    expect(toml).to.include('chunk-request-timeout = "15s"');
    expect(toml).to.include('fetchers = "4"');

    // Light client trust options were removed in Tenderdash 1.7
    expect(toml).to.not.include('trust-height');
    expect(toml).to.not.include('trust-period');

    expect(toml).to.not.include('undefined');
  });

  it('should render statesync consuming disabled', () => {
    config.set('platform.drive.tenderdash.stateSync.enabled', false);

    expect(renderTenderdashConfig()).to.include('enable = false');
  });

  it('should route snapshot serving to the drive grpc app', () => {
    const toml = renderTenderdashConfig();

    expect(toml).to.include('ListSnapshots:grpc:drive_abci:26670');
    expect(toml).to.include('LoadSnapshotChunk:grpc:drive_abci:26670');
    expect(toml).to.include('*:socket:tcp://drive_abci:26658');
    expect(toml).to.include('{ "list_snapshots" = 10 }');
    expect(toml).to.include('{ "load_snapshot_chunk" = 100 }');
  });
});
