import analyseServiceContainersFactory from '../../../../src/doctor/analyse/analyseServiceContainersFactory.js';
import { SEVERITY } from '../../../../src/doctor/Prescription.js';
import Samples from '../../../../src/doctor/Samples.js';

describe('analyseServiceContainersFactory', () => {
  let analyseServiceContainers;
  let samples;

  /**
   * What `si.dockerContainerStats(id)` resolves with: an array holding one
   * entry per requested container, with the Docker stats API's raw CPU and
   * memory objects on it
   *
   * @param {Object} usage
   * @param {number} usage.cpu - share of the system CPU time
   * @param {number} usage.memory - share of the memory limit
   * @return {Object[]}
   */
  function dockerContainerStats({ cpu, memory }) {
    return [{
      id: 'container-id',
      cpuStats: {
        cpu_usage: { total_usage: cpu * 1000000 },
        system_cpu_usage: 1000000,
      },
      memoryStats: {
        usage: memory * 1024,
        limit: 1024,
      },
    }];
  }

  beforeEach(() => {
    samples = new Samples();

    samples.setServiceInfo('core', 'dockerInspect', { State: { Running: true } });

    analyseServiceContainers = analyseServiceContainersFactory(
      () => [{ name: 'core', title: 'Core' }],
    );
  });

  it('should report a service consuming most of the CPU', () => {
    samples.setServiceInfo('core', 'dockerStats', dockerContainerStats({ cpu: 0.9, memory: 0.1 }));

    const problems = analyseServiceContainers(samples);

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.equal('Service Core is consuming 90.00% CPU.');
    expect(problems[0].getSeverity()).to.equal(SEVERITY.MEDIUM);
  });

  it('should report a service consuming most of its memory limit', () => {
    samples.setServiceInfo('core', 'dockerStats', dockerContainerStats({ cpu: 0.1, memory: 0.9 }));

    const problems = analyseServiceContainers(samples);

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.equal('Service Core is consuming 90.00% RAM.');
    expect(problems[0].getSeverity()).to.equal(SEVERITY.MEDIUM);
  });

  it('should report nothing for a service within its resources', () => {
    samples.setServiceInfo('core', 'dockerStats', dockerContainerStats({ cpu: 0.1, memory: 0.1 }));

    expect(analyseServiceContainers(samples)).to.be.empty();
  });

  // Docker had no stats for the container, which says nothing about its load
  it('should report nothing when no container stats were returned', () => {
    samples.setServiceInfo('core', 'dockerStats', []);

    expect(analyseServiceContainers(samples)).to.be.empty();
  });

  // An archive is read back without checking each sample's shape, and one
  // unexpected sample must not cost the operator the rest of the diagnosis
  it('should still analyse the other services when one has stats of an unexpected shape', () => {
    analyseServiceContainers = analyseServiceContainersFactory(() => [
      { name: 'core', title: 'Core' },
      { name: 'drive_abci', title: 'Drive ABCI' },
    ]);

    samples.setServiceInfo('core', 'dockerStats', { cpuStats: {} });
    samples.setServiceInfo('drive_abci', 'dockerInspect', { State: { Running: true } });
    samples.setServiceInfo('drive_abci', 'dockerStats', dockerContainerStats({ cpu: 0.1, memory: 0.9 }));

    const problems = analyseServiceContainers(samples);

    expect(problems.map((problem) => problem.getDescription()))
      .to.deep.equal(['Service Drive ABCI is consuming 90.00% RAM.']);
  });
});
