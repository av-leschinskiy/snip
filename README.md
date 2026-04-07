# snip

Инструмент скриншотов и аннотаций для GNOME/Wayland на Rust.

## Режимы

- `snip` — захват скриншота через XDG Desktop Portal, открытие в редакторе
- `snip edit <path>` — редактирование существующего PNG

## Отладка

```bash
nix develop --command cargo run                # capture mode
nix develop --command cargo run -- edit <path>  # edit mode
```

`nix develop` предоставляет все нативные зависимости (GTK4, cairo, libadwaita). Без него сборка не пройдёт.

## Обновление в NixOS

Snip подключен к nixos-config как flake input с GitHub. Порядок обновления:

```bash
# 1. Закоммитить и запушить изменения
cd ~/projects/snip
git add -A && git commit -m "feat: описание"
git push

# 2. Обновить input в nixos-config и пересобрать систему
cd ~/nixos-config
nix flake update snip
sudo nixos-rebuild switch --flake .
```
