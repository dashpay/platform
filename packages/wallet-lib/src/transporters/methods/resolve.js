const { is } = require('../../utils');

const evonetSeeds = [
  '52.24.198.145',
  '52.13.92.167',
  '34.212.245.91',
];
const palinkaSeeds = [
  '34.214.221.50',
  '54.213.18.11',
  '34.211.149.102',
  '52.38.244.67',
];
const defaultDAPIOpts = {
  seeds: evonetSeeds.map((ip) => ({ service: `${ip}:3000` })),
  timeout: 20000,
  retries: 5,
};
/**
 * Resolves a valid transporter.
 * By default, return a DAPI transporter
 *
 * @param {String|Object|Transporter} props - name of the transporter or options object
 * @param {String} props.type - name of the transporter
 * @param {String} props.devnetName - name of the devnet to connect ('evonet' (def),"palinka")
 * @return {boolean}
 */
module.exports = function resolve(props = { type: 'DAPIClient' }) {
  let opts = {};
  let Transporter = this.getByName('dapi');
  let transporter;
  if (is.string(props)) {
    try {
      Transporter = this.getByName(props);
    } catch (e) {
      console.error('Error:', e.message);
      Transporter = this.getByName('BaseTransporter');
    }
    // TODO: Remove me when DAPIClient has correct seed
    if (Transporter === this.DAPIClient) {
      opts = defaultDAPIOpts;
    }
  } else if (is.obj(props) && props.type) {
    Transporter = this.getByName(props.type || 'dapi');
    // TODO: Remove me when DAPIClient has correct seed
    if (Transporter === this.DAPIClient && !props.seeds) {
      opts = defaultDAPIOpts;
      if (props.devnetName === 'palinka') {
        opts.seeds = palinkaSeeds.map((ip) => ({ service: `${ip}:3000` }));
      }
    }
    opts = Object.assign(opts, props);
  } else {
    if (props === undefined) {
      return resolve('dapi');
    }
    // User may have specified a whole instance of his client.
    if (props.constructor.name !== Function.name) {
      transporter = props;
    }
    // User may have specified a Transporter class that will be validated and used.
    Transporter = props;
  }
  if (!transporter) transporter = new Transporter(opts);
  transporter.isValid = this.validate(transporter);
  return transporter;
};
