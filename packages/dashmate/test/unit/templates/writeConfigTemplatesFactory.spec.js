import writeConfigTemplatesFactory from '../../../src/templates/writeConfigTemplatesFactory.js';

describe('writeConfigTemplatesFactory', () => {
  it('should mark the config saved only after writing its service files', function it() {
    const config = {
      getName: this.sinon.stub().returns('base'),
      markAsSaved: this.sinon.stub(),
    };
    const serviceConfigs = { core: 'rendered' };
    const renderServiceTemplates = this.sinon.stub().returns(serviceConfigs);
    const writeServiceConfigs = this.sinon.stub();
    const writeConfigTemplates = writeConfigTemplatesFactory(
      renderServiceTemplates,
      writeServiceConfigs,
    );

    writeConfigTemplates(config);

    expect(writeServiceConfigs).to.have.been.calledOnceWithExactly('base', serviceConfigs);
    expect(config.markAsSaved).to.have.been.calledOnce();
    expect(writeServiceConfigs).to.have.been.calledBefore(config.markAsSaved);
  });
});
