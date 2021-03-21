const path = require("path");

module.exports = {
  entry: "./static/js/index.js",
  output: {
    path: path.resolve(__dirname, "static/js"),
    filename: "bundle.js",
  },
  devtool: "source-map",
  mode: "development",
  module: {
    rules: [
      {
        test: /\.s[ac]ss$/i,
        use: [
          // Creates `style` nodes from JS strings
          "style-loader",
          // Translates CSS into CommonJS
          "css-loader",
          // Compiles Sass to CSS
          "sass-loader",
        ],
      },
    ],
  },
};
