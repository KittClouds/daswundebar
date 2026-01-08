// vite.config.ts
import { defineConfig } from "file:///C:/Users/shuga/1kittroot/1code/daswundebar/node_modules/vite/dist/node/index.js";
import react from "file:///C:/Users/shuga/1kittroot/1code/daswundebar/node_modules/@vitejs/plugin-react-swc/index.js";
import path from "path";
import { componentTagger } from "file:///C:/Users/shuga/1kittroot/1code/daswundebar/node_modules/lovable-tagger/dist/index.js";
var __vite_injected_original_dirname = "C:\\Users\\shuga\\1kittroot\\1code\\daswundebar";
var vite_config_default = defineConfig(({ mode }) => ({
  server: {
    host: "::",
    port: 8081,
    headers: {
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp"
    }
  },
  plugins: [
    react(),
    mode === "development" && componentTagger(),
    // WASM cache-busting in development
    mode === "development" && {
      name: "wasm-no-cache",
      configureServer(server) {
        server.middlewares.use((req, res, next) => {
          if (req.url?.includes(".wasm")) {
            res.setHeader("Cache-Control", "no-store, no-cache, must-revalidate");
            res.setHeader("Pragma", "no-cache");
          }
          next();
        });
      }
    }
  ].filter(Boolean),
  resolve: {
    alias: {
      "@": path.resolve(__vite_injected_original_dirname, "./src"),
      "@kittcore/wasm": path.resolve(__vite_injected_original_dirname, "./rust/kittcore/pkg")
    }
  },
  optimizeDeps: {
    exclude: ["@sqliteai/sqlite-wasm", "@kittcore/wasm"]
  },
  worker: {
    format: "es"
  },
  build: {
    target: "esnext"
  },
  test: {
    setupFiles: ["./src/test/vitest.setup.ts"],
    environment: "node",
    alias: {
      "cozo-lib-wasm": path.resolve(__vite_injected_original_dirname, "./src/test/__mocks__/cozo-lib-wasm.ts")
    },
    deps: {
      inline: ["cozo-lib-wasm"]
    }
  }
}));
export {
  vite_config_default as default
};
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsidml0ZS5jb25maWcudHMiXSwKICAic291cmNlc0NvbnRlbnQiOiBbImNvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9kaXJuYW1lID0gXCJDOlxcXFxVc2Vyc1xcXFxzaHVnYVxcXFwxa2l0dHJvb3RcXFxcMWNvZGVcXFxcZGFzd3VuZGViYXJcIjtjb25zdCBfX3ZpdGVfaW5qZWN0ZWRfb3JpZ2luYWxfZmlsZW5hbWUgPSBcIkM6XFxcXFVzZXJzXFxcXHNodWdhXFxcXDFraXR0cm9vdFxcXFwxY29kZVxcXFxkYXN3dW5kZWJhclxcXFx2aXRlLmNvbmZpZy50c1wiO2NvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9pbXBvcnRfbWV0YV91cmwgPSBcImZpbGU6Ly8vQzovVXNlcnMvc2h1Z2EvMWtpdHRyb290LzFjb2RlL2Rhc3d1bmRlYmFyL3ZpdGUuY29uZmlnLnRzXCI7aW1wb3J0IHsgZGVmaW5lQ29uZmlnIH0gZnJvbSBcInZpdGVcIjtcclxuaW1wb3J0IHJlYWN0IGZyb20gXCJAdml0ZWpzL3BsdWdpbi1yZWFjdC1zd2NcIjtcclxuaW1wb3J0IHBhdGggZnJvbSBcInBhdGhcIjtcclxuaW1wb3J0IHsgY29tcG9uZW50VGFnZ2VyIH0gZnJvbSBcImxvdmFibGUtdGFnZ2VyXCI7XHJcblxyXG4vLyBodHRwczovL3ZpdGVqcy5kZXYvY29uZmlnL1xyXG5leHBvcnQgZGVmYXVsdCBkZWZpbmVDb25maWcoKHsgbW9kZSB9KSA9PiAoe1xyXG4gIHNlcnZlcjoge1xyXG4gICAgaG9zdDogXCI6OlwiLFxyXG4gICAgcG9ydDogODA4MSxcclxuICAgIGhlYWRlcnM6IHtcclxuICAgICAgXCJDcm9zcy1PcmlnaW4tT3BlbmVyLVBvbGljeVwiOiBcInNhbWUtb3JpZ2luXCIsXHJcbiAgICAgIFwiQ3Jvc3MtT3JpZ2luLUVtYmVkZGVyLVBvbGljeVwiOiBcInJlcXVpcmUtY29ycFwiLFxyXG4gICAgfSxcclxuICB9LFxyXG4gIHBsdWdpbnM6IFtcclxuICAgIHJlYWN0KCksXHJcbiAgICBtb2RlID09PSBcImRldmVsb3BtZW50XCIgJiYgY29tcG9uZW50VGFnZ2VyKCksXHJcbiAgICAvLyBXQVNNIGNhY2hlLWJ1c3RpbmcgaW4gZGV2ZWxvcG1lbnRcclxuICAgIG1vZGUgPT09IFwiZGV2ZWxvcG1lbnRcIiAmJiB7XHJcbiAgICAgIG5hbWU6ICd3YXNtLW5vLWNhY2hlJyxcclxuICAgICAgY29uZmlndXJlU2VydmVyKHNlcnZlcikge1xyXG4gICAgICAgIHNlcnZlci5taWRkbGV3YXJlcy51c2UoKHJlcSwgcmVzLCBuZXh0KSA9PiB7XHJcbiAgICAgICAgICBpZiAocmVxLnVybD8uaW5jbHVkZXMoJy53YXNtJykpIHtcclxuICAgICAgICAgICAgcmVzLnNldEhlYWRlcignQ2FjaGUtQ29udHJvbCcsICduby1zdG9yZSwgbm8tY2FjaGUsIG11c3QtcmV2YWxpZGF0ZScpO1xyXG4gICAgICAgICAgICByZXMuc2V0SGVhZGVyKCdQcmFnbWEnLCAnbm8tY2FjaGUnKTtcclxuICAgICAgICAgIH1cclxuICAgICAgICAgIG5leHQoKTtcclxuICAgICAgICB9KTtcclxuICAgICAgfSxcclxuICAgIH0sXHJcbiAgXS5maWx0ZXIoQm9vbGVhbiksXHJcbiAgcmVzb2x2ZToge1xyXG4gICAgYWxpYXM6IHtcclxuICAgICAgXCJAXCI6IHBhdGgucmVzb2x2ZShfX2Rpcm5hbWUsIFwiLi9zcmNcIiksXHJcbiAgICAgIFwiQGtpdHRjb3JlL3dhc21cIjogcGF0aC5yZXNvbHZlKF9fZGlybmFtZSwgXCIuL3J1c3Qva2l0dGNvcmUvcGtnXCIpLFxyXG4gICAgfSxcclxuICB9LFxyXG4gIG9wdGltaXplRGVwczoge1xyXG4gICAgZXhjbHVkZTogW1wiQHNxbGl0ZWFpL3NxbGl0ZS13YXNtXCIsIFwiQGtpdHRjb3JlL3dhc21cIl0sXHJcbiAgfSxcclxuICB3b3JrZXI6IHtcclxuICAgIGZvcm1hdDogXCJlc1wiLFxyXG4gIH0sXHJcbiAgYnVpbGQ6IHtcclxuICAgIHRhcmdldDogXCJlc25leHRcIixcclxuICB9LFxyXG4gIHRlc3Q6IHtcclxuICAgIHNldHVwRmlsZXM6IFsnLi9zcmMvdGVzdC92aXRlc3Quc2V0dXAudHMnXSxcclxuICAgIGVudmlyb25tZW50OiAnbm9kZScsXHJcbiAgICBhbGlhczoge1xyXG4gICAgICAnY296by1saWItd2FzbSc6IHBhdGgucmVzb2x2ZShfX2Rpcm5hbWUsICcuL3NyYy90ZXN0L19fbW9ja3NfXy9jb3pvLWxpYi13YXNtLnRzJyksXHJcbiAgICB9LFxyXG4gICAgZGVwczoge1xyXG4gICAgICBpbmxpbmU6IFsnY296by1saWItd2FzbSddLFxyXG4gICAgfSxcclxuICB9LFxyXG59KSk7XHJcbiJdLAogICJtYXBwaW5ncyI6ICI7QUFBNFQsU0FBUyxvQkFBb0I7QUFDelYsT0FBTyxXQUFXO0FBQ2xCLE9BQU8sVUFBVTtBQUNqQixTQUFTLHVCQUF1QjtBQUhoQyxJQUFNLG1DQUFtQztBQU16QyxJQUFPLHNCQUFRLGFBQWEsQ0FBQyxFQUFFLEtBQUssT0FBTztBQUFBLEVBQ3pDLFFBQVE7QUFBQSxJQUNOLE1BQU07QUFBQSxJQUNOLE1BQU07QUFBQSxJQUNOLFNBQVM7QUFBQSxNQUNQLDhCQUE4QjtBQUFBLE1BQzlCLGdDQUFnQztBQUFBLElBQ2xDO0FBQUEsRUFDRjtBQUFBLEVBQ0EsU0FBUztBQUFBLElBQ1AsTUFBTTtBQUFBLElBQ04sU0FBUyxpQkFBaUIsZ0JBQWdCO0FBQUE7QUFBQSxJQUUxQyxTQUFTLGlCQUFpQjtBQUFBLE1BQ3hCLE1BQU07QUFBQSxNQUNOLGdCQUFnQixRQUFRO0FBQ3RCLGVBQU8sWUFBWSxJQUFJLENBQUMsS0FBSyxLQUFLLFNBQVM7QUFDekMsY0FBSSxJQUFJLEtBQUssU0FBUyxPQUFPLEdBQUc7QUFDOUIsZ0JBQUksVUFBVSxpQkFBaUIscUNBQXFDO0FBQ3BFLGdCQUFJLFVBQVUsVUFBVSxVQUFVO0FBQUEsVUFDcEM7QUFDQSxlQUFLO0FBQUEsUUFDUCxDQUFDO0FBQUEsTUFDSDtBQUFBLElBQ0Y7QUFBQSxFQUNGLEVBQUUsT0FBTyxPQUFPO0FBQUEsRUFDaEIsU0FBUztBQUFBLElBQ1AsT0FBTztBQUFBLE1BQ0wsS0FBSyxLQUFLLFFBQVEsa0NBQVcsT0FBTztBQUFBLE1BQ3BDLGtCQUFrQixLQUFLLFFBQVEsa0NBQVcscUJBQXFCO0FBQUEsSUFDakU7QUFBQSxFQUNGO0FBQUEsRUFDQSxjQUFjO0FBQUEsSUFDWixTQUFTLENBQUMseUJBQXlCLGdCQUFnQjtBQUFBLEVBQ3JEO0FBQUEsRUFDQSxRQUFRO0FBQUEsSUFDTixRQUFRO0FBQUEsRUFDVjtBQUFBLEVBQ0EsT0FBTztBQUFBLElBQ0wsUUFBUTtBQUFBLEVBQ1Y7QUFBQSxFQUNBLE1BQU07QUFBQSxJQUNKLFlBQVksQ0FBQyw0QkFBNEI7QUFBQSxJQUN6QyxhQUFhO0FBQUEsSUFDYixPQUFPO0FBQUEsTUFDTCxpQkFBaUIsS0FBSyxRQUFRLGtDQUFXLHVDQUF1QztBQUFBLElBQ2xGO0FBQUEsSUFDQSxNQUFNO0FBQUEsTUFDSixRQUFRLENBQUMsZUFBZTtBQUFBLElBQzFCO0FBQUEsRUFDRjtBQUNGLEVBQUU7IiwKICAibmFtZXMiOiBbXQp9Cg==
