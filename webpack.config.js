const path = require('path');

module.exports = {
  entry: './static/js/index.js',
  output: {
    path: path.resolve(__dirname, 'static/js'),
    filename: 'bundle.js'
  },
  devtool: 'source-map'
};