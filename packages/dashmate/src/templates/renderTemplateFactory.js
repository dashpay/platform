import fs from 'fs';
import dots from 'dot';
import crypto from 'crypto';

/**
 * @return {renderTemplate}
 */
export default function renderTemplateFactory() {
  /**
   * Render template for a service
   *
   * @typedef {renderTemplate}
   * @param {string} templatePath
   * @param {object} variables
   *
   * @return {Object<string,string>}
   */
  function renderTemplate(templatePath, variables) {
    const templateString = fs.readFileSync(templatePath, 'utf-8');

    // do not strip \n
    dots.templateSettings.strip = false;

    const template = dots.template(templateString);

    return template({ ...variables, crypto });
  }

  return renderTemplate;
}
