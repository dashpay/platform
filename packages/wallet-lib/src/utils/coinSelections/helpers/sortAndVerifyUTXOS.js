/* eslint-disable no-param-reassign */
const sort = {
  by(el, params) {
    if (!params) return el;

    el.sort((a, b) => {
      let result;
      params.reverse().forEach((param) => {
        const key = param.property;
        const { direction } = (param.direction === 'ascending') ? 1 : -1;

        if ((a[key] < b[key])) {
          result = -1;
        } else {
          result = (a[key] > b[key]) ? 1 : 0;
        }
        return result * direction;
      });
      return 0;
    });

    return el;
  },
};

const sortAndVerifyUTXOS = (utxosList, opts) => sort.by(utxosList, opts);

module.exports = sortAndVerifyUTXOS;
