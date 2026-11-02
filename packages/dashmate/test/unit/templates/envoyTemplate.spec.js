import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import HomeDir from '../../../src/config/HomeDir.js';
import renderServiceTemplatesFactory from '../../../src/templates/renderServiceTemplatesFactory.js';
import renderTemplateFactory from '../../../src/templates/renderTemplateFactory.js';

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
