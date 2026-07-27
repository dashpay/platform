import { expect } from 'chai';
import yaml from 'js-yaml';
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

  it('rejects control characters in host log paths', () => {
    const config = getBaseConfig();

    expect(() => config.set('core.log.filePath', '/tmp/core\nprivileged: true'))
      .to.throw();
    expect(() => config.set(
      'platform.drive.abci.logs.stdout.destination',
      '/tmp/drive\nprivileged: true',
    )).to.throw();
    expect(() => config.set(
      'platform.drive.tenderdash.log.path',
      '/tmp/tenderdash\nprivileged: true',
    )).to.throw();
    expect(() => config.set('platform.gateway.log.accessLogs', [{
      type: 'file',
      format: 'text',
      path: '/tmp/gateway\nprivileged: true',
      template: null,
    }])).to.throw();
  });

  it('serializes host log mounts as scalar values even after a schema bypass', () => {
    const config = getBaseConfig();
    const options = config.getOptions();

    options.core.log.filePath = '/tmp/core\nprivileged: true';
    options.platform.drive.abci.logs.stdout.destination = '/tmp/drive\nprivileged: true';
    options.platform.drive.tenderdash.log.path = '/tmp/tenderdash\nprivileged: true';
    options.platform.gateway.log.accessLogs = [{
      type: 'file',
      format: 'text',
      path: '/tmp/gateway\nprivileged: true',
      template: null,
    }];

    const renderedConfigs = renderServiceTemplates(config);
    const parsed = yaml.load(renderedConfigs['dynamic-compose.yml']);

    for (const service of ['core', 'drive_abci', 'drive_tenderdash', 'gateway']) {
      expect(parsed.services[service]).to.not.have.property('privileged');
      expect(parsed.services[service].volumes).to.have.length(1);
      expect(parsed.services[service].volumes[0]).to.include('\nprivileged: true');
    }
  });
});
