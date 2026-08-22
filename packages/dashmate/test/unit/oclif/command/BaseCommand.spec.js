import fs from 'fs';
import graceful from 'node-graceful';
import HomeDir from '../../../../src/config/HomeDir.js';
import BaseCommand from '../../../../src/oclif/command/BaseCommand.js';
import ResetCommand from '../../../../src/commands/reset.js';
import GroupResetCommand from '../../../../src/commands/group/reset.js';

class MutatingCommand extends BaseCommand {
  static mutatesConfig = true;
}

describe('BaseCommand', () => {
  describe('#init', () => {
    function createCommandWithContainer(sinon, Command = MutatingCommand) {
      const configFile = {
        isChanged: sinon.stub().returns(false),
      };
      const configFileRepository = {
        acquire: sinon.stub(),
        readAndMigrate: sinon.stub().returns({ configFile }),
        release: sinon.stub(),
        write: sinon.stub(),
        isExclusive: () => true,
      };
      const dependencies = {
        configFile,
        configFileRepository,
        startedContainers: {
          getContainers: sinon.stub().returns([]),
        },
        stopAllContainers: sinon.stub().resolves(),
        writeConfigTemplates: sinon.stub(),
      };
      const container = {
        has: sinon.stub().callsFake((name) => name === 'configFile'),
        register: sinon.stub(),
        resolve: sinon.stub().callsFake((name) => dependencies[name]),
      };
      const command = new Command();
      command.parse = sinon.stub().resolves({ args: {}, flags: {} });
      command.createContainer = sinon.stub().resolves(container);

      return {
        command,
        configFileRepository,
        container,
      };
    }

    beforeEach(function beforeEach() {
      this.sinon.stub(graceful, 'on');
    });

    it('should acquire before reading and release after a successful command', async function it() {
      const { command, configFileRepository } = createCommandWithContainer(this.sinon);

      await command.init();

      expect(configFileRepository.acquire).to.have.been.calledOnce();
      expect(configFileRepository.readAndMigrate).to.have.been.calledOnce();
      this.sinon.assert.callOrder(
        configFileRepository.acquire,
        configFileRepository.readAndMigrate,
      );

      await command.finally();

      expect(configFileRepository.release).to.have.been.calledOnce();
    });

    it('should leave service-file repair to explicit config render', async function it() {
      const { command, container } = createCommandWithContainer(this.sinon);

      await command.init();

      expect(container.resolve('writeConfigTemplates')).to.not.have.been.called();
    });

    it('should release after a failed command', async function it() {
      const { command, configFileRepository } = createCommandWithContainer(this.sinon);

      await command.init();
      await command.finally(new Error('command failed'));

      expect(configFileRepository.release).to.have.been.calledOnce();
    });

    it('should release when initialization throws after acquiring', async function it() {
      const { command, configFileRepository } = createCommandWithContainer(this.sinon);
      const initError = new Error('config read failed');
      configFileRepository.readAndMigrate.throws(initError);

      await expect(command.init()).to.be.rejectedWith(initError);
      await command.finally(initError);

      expect(configFileRepository.acquire).to.have.been.calledOnce();
      expect(configFileRepository.release).to.have.been.calledOnce();
    });

    it('should not let an unrelated force flag skip config validation', async function it() {
      const { command, configFileRepository } = createCommandWithContainer(
        this.sinon,
        BaseCommand,
      );
      command.parse.resolves({ args: {}, flags: { force: true } });

      await command.init();

      expect(configFileRepository.readAndMigrate.firstCall.args[0])
        .to.deep.equal({ skipValidation: false });
    });

    it('should skip validation only for the config replaced by a forced total reset', async function it() {
      const platformReset = createCommandWithContainer(this.sinon, ResetCommand);
      platformReset.command.parse.resolves({
        args: {},
        flags: {
          force: true, hard: true, platform: true, config: 'base',
        },
      });

      await platformReset.command.init();

      expect(platformReset.configFileRepository.readAndMigrate.firstCall.args[0])
        .to.deep.equal({ skipValidation: false });

      const totalReset = createCommandWithContainer(this.sinon, ResetCommand);
      totalReset.command.parse.resolves({
        args: {},
        flags: {
          force: true, hard: true, platform: false, config: 'base',
        },
      });

      await totalReset.command.init();

      const { skipValidation } = totalReset.configFileRepository.readAndMigrate.firstCall.args[0];

      expect(skipValidation).to.be.a('function');
      expect(skipValidation({ name: 'base', configFileData: {} })).to.be.true();
      expect(skipValidation({ name: 'node1', configFileData: {} })).to.be.false();
    });

    it('should skip validation only for configs replaced by a forced group reset', async function it() {
      const groupReset = createCommandWithContainer(this.sinon, GroupResetCommand);
      groupReset.command.parse.resolves({
        args: {},
        flags: {
          force: true, hard: true, platform: false, group: 'local',
        },
      });

      await groupReset.command.init();

      const { skipValidation } = groupReset.configFileRepository.readAndMigrate.firstCall.args[0];

      expect(skipValidation).to.be.a('function');
      expect(skipValidation({ options: { group: 'local' }, configFileData: {} })).to.be.true();
      expect(skipValidation({ options: { group: 'other' }, configFileData: {} })).to.be.false();
    });

    it('should keep the command lease while graceful container cleanup runs', async function it() {
      const homeDir = HomeDir.createTemp();
      const previousHomeDir = process.env.DASHMATE_HOME_DIR;
      let repository;
      let release;

      try {
        process.env.DASHMATE_HOME_DIR = homeDir.getPath();

        let exitHandler;
        graceful.on.callsFake((event, handler) => {
          if (event === 'exit') {
            exitHandler = handler;
          }
        });

        const command = new MutatingCommand();
        command.parse = this.sinon.stub().resolves({ args: {}, flags: {} });

        await command.init();

        repository = command.container.resolve('configFileRepository');
        release = this.sinon.spy(repository, 'release');

        await exitHandler();

        expect(release).to.not.have.been.called();
        expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.true();
      } finally {
        if (release) {
          release.wrappedMethod.call(repository);
        }

        if (previousHomeDir === undefined) {
          delete process.env.DASHMATE_HOME_DIR;
        } else {
          process.env.DASHMATE_HOME_DIR = previousHomeDir;
        }

        homeDir.remove();
      }
    });
  });

  describe('#saveConfigAndStopContainers', () => {
    let command;
    let config;
    let configFile;
    let configFileRepository;
    let writeConfigTemplates;
    let stopAllContainers;
    let startedContainers;

    beforeEach(function beforeEach() {
      config = {
        isChanged: this.sinon.stub().returns(true),
      };
      configFile = {
        isChanged: this.sinon.stub().returns(true),
        getAllConfigs: this.sinon.stub().returns([config]),
      };
      configFileRepository = {
        write: this.sinon.stub(),
        isExclusive: () => true,
      };
      writeConfigTemplates = this.sinon.stub();
      stopAllContainers = this.sinon.stub().resolves();
      startedContainers = {
        getContainers: this.sinon.stub().returns([]),
      };

      const dependencies = {
        configFile,
        configFileRepository,
        startedContainers,
        stopAllContainers,
        writeConfigTemplates,
      };

      command = new MutatingCommand();
      command.container = {
        has: this.sinon.stub().callsFake((name) => name === 'configFile'),
        resolve: this.sinon.stub().callsFake((name) => dependencies[name]),
      };
    });

    it('should keep the saved config when template rendering fails', async function it() {
      configFileRepository.write.callsFake(() => {
        configFile.isChanged.returns(false);
      });
      writeConfigTemplates.onFirstCall().throws(new Error('template write failed'));

      await expect(command.saveConfigAndStopContainers())
        .to.be.rejectedWith('template write failed');

      expect(configFileRepository.write).to.have.been.calledOnceWith(configFile);
      expect(writeConfigTemplates).to.have.been.calledOnceWith(config);
      this.sinon.assert.callOrder(configFileRepository.write, writeConfigTemplates);
    });

    // Check exclusivity before saving so a command whose lock was stolen does
    // not persist or render its stale snapshot. With a live lease, JSON is
    // saved before the derived service files are rendered.
    it('should not render service files once the lock has been lost', async function it() {
      configFileRepository.isExclusive = () => false;
      // What saving does when the lease is gone: refuse, but first put the
      // pending configuration somewhere the operator can get it back from.
      configFileRepository.write.throws(new Error('Lost the lock; rescue written'));

      await expect(command.saveConfigAndStopContainers())
        .to.be.rejectedWith('Lost the lock; rescue written');

      // The rescue is the only copy of work a completed setup already did out
      // in the world, so the save has to be attempted even though it fails.
      expect(configFileRepository.write).to.have.been.calledOnceWith(configFile);

      // Rendering would overwrite service files the process that took the lock
      // has already written from newer state.
      expect(writeConfigTemplates).to.not.have.been.called();
    });

    it('should stop started containers when a refused config save throws', async function it() {
      configFileRepository.write.throws(new Error('Lost the lock; rescue written'));

      await expect(command.saveConfigAndStopContainers())
        .to.be.rejectedWith('Lost the lock; rescue written');

      expect(stopAllContainers).to.have.been.calledOnceWith([], { remove: true });
    });

    it('should not save the startup config for a non-mutating command', async function it() {
      const nonMutatingCommand = new BaseCommand();
      nonMutatingCommand.container = command.container;

      await nonMutatingCommand.saveConfigAndStopContainers();

      expect(configFileRepository.write).to.not.have.been.called();
      expect(writeConfigTemplates).to.not.have.been.called();
    });
  });
});
