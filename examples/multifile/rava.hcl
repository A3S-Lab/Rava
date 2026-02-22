project {
  name    = "multifile-example"
  version = "0.1.0"
  java    = "21"
}

build {
  target   = "native"
  main     = "Main"
  optimize = "speed"
}
