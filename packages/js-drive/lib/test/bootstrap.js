/* eslint-disable */
const path = require('path');
const dotenvSafe = require('dotenv-safe');
const dotenvExpand = require('dotenv-expand');
const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const chaiString = require('chai-string');

const lodash = require('lodash');

const DashCoreOptions = require('@dashevo/dp-services-ctl/lib/services/dashCore/DashCoreOptions');

const { default: loadWasmDpp, Identifier } = require('@dashevo/wasm-dpp');

const getBlsAdapter = require('../bls/getBlsAdapter');

use(sinonChai);
use(chaiAsPromised);
use(chaiString);
use(dirtyChai);
use(async (chai, util) => {
  await loadWasmDpp();

  const getMofifiedArgument = (argument) => {
    if (!argument.hasOwnProperty('ptr')) {
      return argument;
    }

    const isIdentifier = (argument instanceof Identifier);

    if (isIdentifier) {
      return argument.toBuffer();
    }

    try {
      return argument.toJSON();
    } catch (e) {
      try {
        return argument.serialize();
      } catch (se) {
        try {
          return argument.toBuffer();
        } catch (e) { }
      }
    }

    return argument;
  };

  const transformArguments = (args, topLevelObject, basePath = '') => {
    lodash.forIn(args, (value, key) => {
      if (value !== undefined && value !== null && value.hasOwnProperty('ptr')) {
        lodash.set(topLevelObject, `${basePath}${basePath === '' ? '' : '.'}${key}`, getMofifiedArgument(value));
        return;
      }

      if (lodash.isArray(value)) {
        value.forEach((item, index) => {
          if (lodash.isObject(item)) {
            if (item !== undefined && item !== null && item.hasOwnProperty('ptr')) {
              lodash.set(topLevelObject, `${basePath}${basePath === '' ? '' : '.'}${key}[${index}]`, getMofifiedArgument(item));
              return;
            }

            transformArguments(item, topLevelObject);
          }
        });
      }

      if (lodash.isObject(value)) {
        transformArguments(value, topLevelObject, `${basePath}${basePath === '' ? '' : '.'}${key}`);
      }
    });
  };

  // eslint-disable-next-line
  chai.Assertion.overwriteMethod('equals', function (_super) {
    return function (other) {
      const originalObject = {
        0: this._obj,
      };

      transformArguments(originalObject, originalObject);
      transformArguments(arguments, arguments);

      new chai.Assertion(originalObject['0']).to.deep.equal(arguments['0']);
    };
  });

  // eslint-disable-next-line
  chai.Assertion.overwriteMethod('calledOnceWithExactly', function (_super) {
    return function () {
      const clonedCallArgs = lodash.cloneDeep(
        this._obj.getCall(0).args.reduce((obj, next, index) => ({
          ...obj,
          [index]: next,
        }), {}),
      );
      transformArguments(clonedCallArgs, clonedCallArgs);

      const clonedArgs = lodash.cloneDeep(arguments);
      transformArguments(clonedArgs, clonedArgs);

      new chai.Assertion(this._obj.callCount).to.equal(1);
      new chai.Assertion(clonedCallArgs).to.deep.equal(clonedArgs);
    };
  });
});

process.env.NODE_ENV = 'test';

// Workaround for dotenv-safe
if (process.env.INITIAL_CORE_CHAINLOCKED_HEIGHT === undefined) {
  process.env.INITIAL_CORE_CHAINLOCKED_HEIGHT = 0;
}
if (process.env.DPNS_MASTER_PUBLIC_KEY === undefined) {
  process.env.DPNS_MASTER_PUBLIC_KEY = '037d074eb00aa286c438b5d12b7c6ca25104d61b03e6601b6ace7d5eb036fbbc23';
}
if (process.env.DPNS_SECOND_PUBLIC_KEY === undefined) {
  process.env.DPNS_SECOND_PUBLIC_KEY = '025852df611a228b0e7fbccff4eaa117500ead84622809ea7fc05dcf6d2dbbc1d4';
}
if (process.env.DASHPAY_MASTER_PUBLIC_KEY === undefined) {
  process.env.DASHPAY_MASTER_PUBLIC_KEY = '02c571ff0cdb72634de4fd23f40c4ed530b3d31defc987a55479d65e7e8c1e249a';
}
if (process.env.DASHPAY_SECOND_PUBLIC_KEY === undefined) {
  process.env.DASHPAY_SECOND_PUBLIC_KEY = '03834f92a2132e55273cb713e855a6fbf2179704830c19094470720d6434ce4547';
}
if (process.env.FEATURE_FLAGS_MASTER_PUBLIC_KEY === undefined) {
  process.env.FEATURE_FLAGS_MASTER_PUBLIC_KEY = '022393486382a5bb262856c49f869827a1c79a3a3c38747f3cb8c32dd7bd191797';
}
if (process.env.FEATURE_FLAGS_SECOND_PUBLIC_KEY === undefined) {
  process.env.FEATURE_FLAGS_SECOND_PUBLIC_KEY = '03c10ac08a77dfdfcdc706ea43d9651ac0866181b835411587eca4d2d5477f39f7';
}
if (process.env.MASTERNODE_REWARD_SHARES_MASTER_PUBLIC_KEY === undefined) {
  process.env.MASTERNODE_REWARD_SHARES_MASTER_PUBLIC_KEY = '0333a42c628e8c93ce0386856f8f2239c84bf816cf8590716c7891fdc981a4df0b';
}
if (process.env.MASTERNODE_REWARD_SHARES_SECOND_PUBLIC_KEY === undefined) {
  process.env.MASTERNODE_REWARD_SHARES_SECOND_PUBLIC_KEY = '03a3002856ad91662dc34b6650a2c7f8b2726c947a419e0b880fb3acc38763a271';
}
if (process.env.WITHDRAWALS_MASTER_PUBLIC_KEY === undefined) {
  process.env.WITHDRAWALS_MASTER_PUBLIC_KEY = '02ee6d9b15ed1c310535297739e69973406dbd1a679be7fad2bdd2e08685033077';
}
if (process.env.WITHDRAWALS_SECOND_PUBLIC_KEY === undefined) {
  process.env.WITHDRAWALS_SECOND_PUBLIC_KEY = '02711ce1fedafde67694d771950c474fe300e464d4f67c2a6447f9b61e90fa01f3';
}

const dotenvConfig = dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});

dotenvExpand(dotenvConfig);

DashCoreOptions.setDefaultCustomOptions({
  container: {
    image: 'dashpay/dashd:18.1.0-rc.1',
  },
});

before(async function before() {
  this.blsAdapter = await getBlsAdapter();
  await loadWasmDpp();
});

beforeEach(function beforeEach() {
  if (!this.sinon) {
    this.sinon = sinon.createSandbox();
  } else {
    this.sinon.restore();
  }
});

afterEach(function afterEach() {
  this.sinon.restore();
});

global.expect = expect;
