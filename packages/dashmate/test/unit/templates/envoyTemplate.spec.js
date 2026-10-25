import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import HomeDir from '../../../src/config/HomeDir.js';
import renderServiceTemplatesFactory from '../../../src/templates/renderServiceTemplatesFactory.js';
import renderTemplateFactory from '../../../src/templates/renderTemplateFactory.js';

describe('envoy template', () => {
  let config;

  beforeEach(() => {
    const getBaseConfig = getBaseConfigFactory(HomeDir.createTemp());
    config = getBaseConfig();
  });

  function renderEnvoyConfig() {
    const renderTemplate = renderTemplateFactory();
    const renderServiceTemplates = renderServiceTemplatesFactory(renderTemplate);

    return renderServiceTemplates(config)['platform/gateway/envoy.yaml'];
  }

  it('should render admin interface when metrics are enabled even if admin is disabled', () => {
    config.set('platform.gateway.metrics.enabled', true);
    config.set('platform.gateway.admin.enabled', false);

    const envoyConfig = renderEnvoyConfig();

    expect(envoyConfig).to.include('cluster_name: admin');
    expect(envoyConfig).to.include('address: 127.0.0.1');
    expect(envoyConfig).to.include('port_value: 9901');
  });

  it('should gzip only grpc-web responses, after grpc_web has framed them', () => {
    const envoyConfig = renderEnvoyConfig();

    // Encoders run in reverse filter order, so a compressor listed between cors and
    // grpc_web compresses the final grpc-web body, trailer frame included.
    const cors = envoyConfig.indexOf('- name: envoy.filters.http.cors');
    const compressor = envoyConfig.indexOf('- name: envoy.filters.http.compressor');
    const grpcWeb = envoyConfig.indexOf('- name: envoy.filters.http.grpc_web');

    expect(cors).to.be.greaterThan(-1);
    expect(compressor).to.be.greaterThan(cors);
    expect(grpcWeb).to.be.greaterThan(compressor);

    // Native gRPC must never be HTTP-compressed: rs-dapi compresses its messages.
    const contentTypes = envoyConfig
      .slice(compressor, grpcWeb)
      .match(/^\s+- application\/[\w+-]+$/gm)
      .map((line) => line.trim().slice(2));

    expect(contentTypes).to.have.members([
      'application/grpc-web',
      'application/grpc-web+proto',
      'application/grpc-web-text',
      'application/grpc-web-text+proto',
    ]);
  });
});
