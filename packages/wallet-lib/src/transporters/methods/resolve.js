const { is } = require('../../utils');

const defaultDAPIOpts = {
  seeds: [
    { service: 'seed-1.evonet.networks.dash.org' },
    { service: 'seed-2.evonet.networks.dash.org' },
    { service: 'seed-3.evonet.networks.dash.org' },
    { service: 'seed-4.evonet.networks.dash.org' },
    { service: 'seed-5.evonet.networks.dash.org' },
  ],
  timeout: 20000,
  retries: 5,
};
/**
 * Resolves a valid transporter.
 * By default, return a DAPI transporter
 *
 * @param {String|Object|Transporter} props - name of the transporter or options object
 * @param {String} props.type - name of the transporter
 * @param {String} props.devnetName - name of the devnet to connect ('evonet' (def))
 * @return {Transporter}
 */
module.exports = function resolve(props = { type: 'DAPIClient' }) {
  let opts = {};
  let Transporter = this.getByName('dapi');
  let transporter;
  if (is.string(props)) {
    try {
      Transporter = this.getByName(props);
    } catch (e) {
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
