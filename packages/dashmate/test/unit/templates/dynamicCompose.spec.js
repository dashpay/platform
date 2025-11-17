import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import renderTemplateFactory from '../../../src/templates/renderTemplateFactory.js';
import renderServiceTemplatesFactory from '../../../src/templates/renderServiceTemplatesFactory.js';

function getRsDapiBlock(dynamicComposeContent) {
  const match = dynamicComposeContent.match(/rs_dapi:\n((?: {2}.*\n)+)/);
  return match ? match[1] : '';
}

describe('dynamic compose template', () => {
  let getBaseConfig;
  let renderServiceTemplates;

  beforeEach(() => {
    getBaseConfig = getBaseConfigFactory(HomeDir.createTemp());
    const renderTemplate = renderTemplateFactory();
    renderServiceTemplates = renderServiceTemplatesFactory(renderTemplate);
  });

  it('should not publish metrics port when rs-dapi metrics are disabled', () => {
    const config = getBaseConfig();

    const renderedConfigs = renderServiceTemplates(config);
    const rsDapiBlock = getRsDapiBlock(renderedConfigs['dynamic-compose.yml']);

    expect(rsDapiBlock).to.not.include('ports:');
    expect(rsDapiBlock).to.not.include(':0');
  });

  it('should publish metrics port when rs-dapi metrics are enabled', () => {
    const config = getBaseConfig();

    config.set('platform.dapi.rsDapi.metrics.enabled', true);
    config.set('platform.dapi.rsDapi.metrics.port', 29091);
    config.set('platform.dapi.rsDapi.metrics.host', '127.0.0.1');

    const renderedConfigs = renderServiceTemplates(config);
    const rsDapiBlock = getRsDapiBlock(renderedConfigs['dynamic-compose.yml']);

    expect(rsDapiBlock).to.include('ports:\n      - 127.0.0.1:29091:29091');
    expect(rsDapiBlock).to.include('- 29091');
  });
});
