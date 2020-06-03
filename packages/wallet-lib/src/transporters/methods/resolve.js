const { is } = require('../../utils');


const defaultTransporterType = 'DAPIClientWrapper';
/**
 * A number, or a string containing a number.
 * @typedef {(DAPIClient|DAPIClientWrapper|RPCClient|ProtocolClient)} Transporter
 */

/**
 * Resolves a valid transporter.
 * By default, return a DAPI transporter
 * @param {string|object|function|Transporter} [props] - name of the transporter or options object
 * @param {string} props.type - name of the transporter
 * @return {object}
 */
module.exports = function resolve(props = { type: defaultTransporterType }) {
  // Used to hold a transporter constructor
  let Transporter;

  // If an instance is created, will be hold in order to be validated
  let transporter;
  if (!props) {
    throw new Error('Unexpected null parameter');
  }

  const isConstructor = !!props.prototype;
  // We passed a constructor that we expect being returned as instance so we init it.
  if (isConstructor) {
    Transporter = props;
    transporter = new Transporter();
  } else {
    const isObjectProps = props.constructor === Object;
    // We allow to pass the name in props.type
    if (isObjectProps) {
      Transporter = this.getByName(props.type || defaultTransporterType);
      transporter = new Transporter(props);
    } else if (!is.string(props)) {
      // Then it's simply an already initialized instance that we return;
      transporter = props;
    } else {
      // At this point, props can only be string
      Transporter = this.getByName(props);
      transporter = new Transporter(props);
    }
  }

  // Validation is helpful to inform dev about missing needed key and for Account
  // to know if it's should default on offlineMode and avoid trying use transport layer
  transporter.isValid = this.validate(transporter);
  return transporter;
};
