{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  name = "stockfin";

  nativeBuildInputs = with pkgs; [
    pkg-config
    wrapGAppsHook4
    gobject-introspection
  ];

  buildInputs = with pkgs; [
    gtk4
    graphene.dev
    glib.dev
    pango
    libadwaita
    adwaita-icon-theme
    gsettings-desktop-schemas
  ];
}
