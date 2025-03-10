{ pkgs, lib, config, inputs, ... }:

{
  packages = [ 
  	pkgs.git
	pkgs.git-lfs
  ];

  languages.rust = {
	enable = true;
	channel = "stable";
  };

  enterShell = ''
    echo "~~~ fuzzija ~~~"
  '';

  enterTest = ''
    echo "Running tests"
  '';
}
