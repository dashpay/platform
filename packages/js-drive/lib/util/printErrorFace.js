const chalk = require('chalk');

// Faces https://github.com/maxogden/cool-ascii-faces
const faces = [
  '\\_(ʘ_ʘ)_/',
  '(•̀o•́)ง',
  'ヽ༼° ͟ل͜ ͡°༽ﾉ',
  'ノ( ゜-゜ノ)',
  '༼ ºل͟º ༽',
  '(ಥ﹏ಥ)',
  '¯\\_(ツ)_/¯',
  '(╯°□°)╯︵ ┻━┻', // https://looks.wtf/flipping-tables
];

/**
 * @return {string}
 */
function printErrorFace() {
  let face = '';

  // top padding
  face += '\n\n';

  // face
  face += chalk.red(
    faces[Math.floor(Math.random() * faces.length)],
  );

  // bottom padding
  face += '\n\n';

  return face;
}

module.exports = printErrorFace;
