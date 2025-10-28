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
    expect(envoyConfig).to.include('admin:\n address:\n   socket_address:\n     address: 127.0.0.1 # Admin listens on configured host; defaults to loopback when not explicitly enabled');
    expect(envoyConfig).to.include('port_value: 9901');
  });
});
