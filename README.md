# mylovepdf

Binaire PDF tout-en-un en ligne de commande, façon iLovePDF, écrit en Rust.

## Fonctionnalités

1. **Compresser** un PDF
2. **Split** (découper) un PDF en plusieurs fichiers
3. **Fusionner** plusieurs PDF en un seul
4. **Convertir** des images JPG/PNG en PDF
5. **Convertir** un PDF en images JPG
6. **Éditer** un PDF dans Xournal++ (annotation)

## Compilation

```bash
cargo build --release
```

Le binaire se trouve dans `target/release/mylovepdf`.

## Utilisation

### Fusionner des PDF

```bash
mylovepdf merge a.pdf b.pdf c.pdf -o resultat.pdf
```

### Découper un PDF

Toutes les pages :

```bash
mylovepdf split document.pdf -o sortie/
```

Une seule page (index 1-based) :

```bash
mylovepdf split document.pdf -o sortie/ --page 3
```

### Compresser un PDF

```bash
mylovepdf compress document.pdf -o compresse.pdf
```

### Convertir des images en PDF

```bash
mylovepdf images-to-pdf photo1.jpg photo2.png -o images.pdf
```

### Convertir un PDF en JPG

```bash
mylovepdf pdf-to-jpg document.pdf -o images/ --dpi 150
```

`--dpi` contrôle la résolution de sortie (défaut 150).

### Éditer un PDF dans Xournal++

```bash
mylovepdf edit document.pdf
```

Ouvre le PDF dans [Xournal++](https://xournalpp.github.io/) pour l'annoter. Si `xournalpp` n'est pas installé, la commande le détecte et propose de l'installer via le gestionnaire de paquets du système (apt, dnf, pacman, etc.).

## Aide

Chaque commande dispose d'une aide détaillée :

```bash
mylovepdf --help
mylovepdf merge --help
```

## Dépendances

- [clap](https://crates.io/crates/clap) — interface en ligne de commande
- [lopdf](https://crates.io/crates/lopdf) — manipulation des PDF (fusion, split, compression, images)
- [pdf-render](https://crates.io/crates/pdf-render) — rendu PDF vers image (conversion PDF → JPG)
- [image](https://crates.io/crates/image) — encodage/décodage JPG et PNG
