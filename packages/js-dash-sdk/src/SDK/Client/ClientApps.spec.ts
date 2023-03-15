import Identifier from '@dashevo/dpp/lib/Identifier';
import { expect } from 'chai';
import { ClientApps } from './ClientApps';
import 'mocha';

describe('ClientApps', () => {
  let apps;
  let appsFromProps;
  it('constructor', () => {
    apps = new ClientApps();
    expect(apps.apps).to.deep.equal({});
    appsFromProps = new ClientApps({
      dpns: {
        contractId: '3VvS19qomuGSbEYWbTsRzeuRgawU3yK4fPMzLrbV62u8',
        contract: null,
      },
    });
  });
  it('.set', () => {
    apps.set('dpns', {
      contractId: '3VvS19qomuGSbEYWbTsRzeuRgawU3yK4fPMzLrbV62u8',
      contract: { someField: true },
    });
    apps.set('tutorialContract', {
      contractId: '3VvS19qomuGSbEYWbTsRzeuRgawU3yK4fPMzLrbV62u8',
      contract: { someField: true },
    });
  });
  it('should get', () => {
    const getByName = apps.get('dpns');
    expect(getByName).to.deep.equal({
      contractId: Identifier.from('3VvS19qomuGSbEYWbTsRzeuRgawU3yK4fPMzLrbV62u8'),
      contract: { someField: true },
    });
  });

  it('should .getNames()', () => {
    const names = apps.getNames();
    expect(names).to.deep.equal(['dpns', 'tutorialContract']);
  });
  it('should .has', () => {
    expect(apps.has('dpns')).to.equal(true);
    expect(apps.has('tutorialContract')).to.equal(true);
    expect(apps.has('tutorialContractt')).to.equal(false);
  });
});
