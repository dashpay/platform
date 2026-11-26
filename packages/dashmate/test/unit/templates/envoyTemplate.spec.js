import yaml from 'js-yaml';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import HomeDir from '../../../src/config/HomeDir.js';
import renderServiceTemplatesFactory from '../../../src/templates/renderServiceTemplatesFactory.js';
import renderTemplateFactory from '../../../src/templates/renderTemplateFactory.js';

const GET_BLOCKS_PATH = '/org.dash.platform.dapi.v0.Platform/getBlocks';

// envoy.yaml opens with `!ignore filters: &filters`, a tag only Envoy's own loader knows
const ENVOY_YAML_SCHEMA = yaml.DEFAULT_SCHEMA.extend(new yaml.Type('!ignore', { kind: 'scalar' }));

function renderGatewayConfigs(customize = () => {}) {
  const config = getBaseConfigFactory(HomeDir.createTemp())();
  customize(config);

  const renderTemplate = renderTemplateFactory();
  return renderServiceTemplatesFactory(renderTemplate)(config);
}

function findRoutes(node) {
  if (node === null || typeof node !== 'object') {
    return undefined;
  }
  if (node.virtual_hosts) {
    return node.virtual_hosts[0].routes;
  }
  return Object.values(node).reduce((found, child) => found ?? findRoutes(child), undefined);
}

function loadEnvoyRoutes(renderedConfigs) {
  return findRoutes(yaml.load(renderedConfigs['platform/gateway/envoy.yaml'], { schema: ENVOY_YAML_SCHEMA }));
}

function loadGetBlocksDescriptors(renderedConfigs) {
  const { descriptors } = yaml.load(renderedConfigs['platform/gateway/rate_limiter/rate_limiter.yaml']);
  return descriptors.find(({ key, value }) => key === 'method' && value === 'getBlocks').descriptors;
}

describe('gateway rate limit of getBlocks', () => {
  it('should send getBlocks through its own limit before the Platform prefix route', () => {
    const routes = loadEnvoyRoutes(renderGatewayConfigs());

    const routeIndex = routes.findIndex(({ match }) => match.path === GET_BLOCKS_PATH);
    const prefixIndex = routes.findIndex(({ match }) => (
      match.prefix === '/org.dash.platform.dapi.v0.Platform'
    ));

    expect(routeIndex).to.be.at.least(0);
    expect(routeIndex).to.be.below(prefixIndex);
    expect(routes[routeIndex].route.cluster).to.equal('rs_dapi');
    expect(routes[routeIndex].route.rate_limits).to.deep.equal([{
      actions: [
        { generic_key: { descriptor_key: 'method', descriptor_value: 'getBlocks' } },
        { remote_address: {} },
      ],
    }]);
  });

  it('should allow one IP far fewer getBlocks requests than other requests', () => {
    const descriptors = loadGetBlocksDescriptors(renderGatewayConfigs());

    expect(descriptors).to.deep.equal([{
      key: 'remote_address',
      rate_limit: { unit: 'hour', requests_per_unit: 30 },
    }]);
  });

  it('should keep the blacklist and whitelist in force for getBlocks', () => {
    const descriptors = loadGetBlocksDescriptors(renderGatewayConfigs((config) => {
      config.set('platform.gateway.rateLimiter.blacklist', ['1.1.1.1']);
      config.set('platform.gateway.rateLimiter.whitelist', ['2.2.2.2']);
    }));

    expect(descriptors).to.deep.include({
      key: 'remote_address',
      value: '1.1.1.1',
      rate_limit: { unit: 'second', requests_per_unit: 0 },
    });
    expect(descriptors).to.deep.include({
      key: 'remote_address',
      value: '2.2.2.2',
      rate_limit: { unlimited: true },
    });
  });

  it('should not reference the rate limiter from the route when it is disabled', () => {
    const routes = loadEnvoyRoutes(renderGatewayConfigs((config) => {
      config.set('platform.gateway.rateLimiter.enabled', false);
    }));
    const route = routes.find(({ match }) => match.path === GET_BLOCKS_PATH);

    expect(route.route).to.not.have.property('rate_limits');
  });
});

describe('envoy template', () => {
  it('should render admin interface when metrics are enabled even if admin is disabled', () => {
    const getBaseConfig = getBaseConfigFactory(HomeDir.createTemp());
    const config = getBaseConfig();

    config.set('platform.gateway.metrics.enabled', true);
    config.set('platform.gateway.admin.enabled', false);

    const renderTemplate = renderTemplateFactory();
    const renderServiceTemplates = renderServiceTemplatesFactory(renderTemplate);
    const renderedConfigs = renderServiceTemplates(config);

    const envoyConfig = renderedConfigs['platform/gateway/envoy.yaml'];

    expect(envoyConfig).to.include('cluster_name: admin');
    expect(envoyConfig).to.include('address: 127.0.0.1');
    expect(envoyConfig).to.include('port_value: 9901');
  });

  it('should gzip only grpc-web responses, after grpc_web has framed them', () => {
    const getBaseConfig = getBaseConfigFactory(HomeDir.createTemp());
    const config = getBaseConfig();

    const renderTemplate = renderTemplateFactory();
    const renderServiceTemplates = renderServiceTemplatesFactory(renderTemplate);
    const envoyConfig = renderServiceTemplates(config)['platform/gateway/envoy.yaml'];

    // Encoders run in reverse filter order, so a compressor listed between cors and
    // grpc_web compresses the final grpc-web body, trailer frame included.
    const cors = envoyConfig.indexOf('- name: envoy.filters.http.cors');
    const compressor = envoyConfig.indexOf('- name: envoy.filters.http.compressor');
    const grpcWeb = envoyConfig.indexOf('- name: envoy.filters.http.grpc_web');

    expect(cors).to.be.greaterThan(-1);
    expect(compressor).to.be.greaterThan(cors);
    expect(grpcWeb).to.be.greaterThan(compressor);

    // Only grpc-web responses are compressed; rs-dapi compresses native gRPC itself.
    const responses = envoyConfig.indexOf('response_direction_config:', compressor);
    const library = envoyConfig.indexOf('compressor_library:', compressor);

    expect(responses).to.be.within(compressor, library);

    const contentTypes = (envoyConfig
      .slice(responses, library)
      .match(/^\s+- application\/[\w+-]+$/gm) ?? [])
      .map((line) => line.trim().slice(2));

    expect(contentTypes).to.have.members([
      'application/grpc-web',
      'application/grpc-web+proto',
      'application/grpc-web-text',
      'application/grpc-web-text+proto',
    ]);
  });
});
