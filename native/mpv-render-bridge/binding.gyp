{
  "targets": [
    {
      "target_name": "mpv_render_bridge",
      "sources": ["src/addon.cc"],
      "cflags_cc": [
        "-std=c++17",
        "<!@(pkg-config --cflags mpv)"
      ],
      "libraries": [
        "<!@(pkg-config --libs mpv)"
      ]
    }
  ]
}
