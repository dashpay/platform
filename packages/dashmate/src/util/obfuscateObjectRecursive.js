export default function obfuscateObjectRecursive(object, func) {
  for (const objectField in object) {
    if (Object.hasOwn(object, objectField)) {
      if (typeof object[objectField] === 'object') {
        obfuscateObjectRecursive(object[objectField], func);
      } else {
        // eslint-disable-next-line no-param-reassign
        object[objectField] = func(objectField, object[objectField]);
      }
    }
  }
}
