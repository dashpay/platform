import lodash from 'lodash';

const hideString = (string) => (typeof string === 'string'
  ? Array.from(string).map(() => '*').join('')
  : string);

export default function sanitizeConfig(
  config,
) {
  const sanitizeFieldRecursive = (object, field) => {
    for (const objectField in object) {
      if (typeof object[objectField] === 'object') {
        sanitizeFieldRecursive(object[objectField], field);
      } else if (objectField === field) {
        // replace all symbols with *
        // eslint-disable-next-line no-param-reassign
        object[field] = hideString(object[objectField]);
      }
    }
  };

  const cloned = lodash.cloneDeep(config);

  sanitizeFieldRecursive(cloned, 'password');
  sanitizeFieldRecursive(cloned, 'apiKey');
  sanitizeFieldRecursive(cloned, 'privateKey');
  sanitizeFieldRecursive(cloned, 'externalIp');

  return cloned;
}
