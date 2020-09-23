{ stdenv, cargo, raspberrypi-tools, gcc, pkgsBuildBuild, pkgconfig, rustfmt }: stdenv.mkDerivation {
  name = "hyperpixel-init";
  version = "unstable";
  src = ./.;
  nativeBuildInputs = [ rustfmt cargo gcc pkgconfig ];
  depsBuildBuild = [ gcc ];
  buildInputs = [ raspberrypi-tools ];
  LIBCLANG_PATH = "${pkgsBuildBuild.llvmPackages.libclang}/lib";
  STDLIB_PATH = "${gcc.libc.dev}/include";
}
