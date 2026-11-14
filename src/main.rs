use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(
    name = "mylovepdf",
    version,
    about = "Binaire PDF tout-en-un : compresser, split, fusionner, convertir images<->PDF"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fusionner plusieurs PDF en un seul
    Merge {
        /// Fichiers PDF d'entrée (au moins 2)
        #[arg(required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        /// Fichier de sortie
        #[arg(short, long, default_value = "merged.pdf")]
        output: PathBuf,
    },
    /// Découper un PDF en plusieurs fichiers
    Split {
        /// Fichier PDF d'entrée
        input: PathBuf,
        /// Répertoire de sortie
        #[arg(short, long, default_value = "split")]
        output_dir: PathBuf,
        /// Extraire une seule page (index 1-based)
        #[arg(long)]
        page: Option<u32>,
    },
    /// Compresser un PDF
    Compress {
        /// Fichier PDF d'entrée
        input: PathBuf,
        /// Fichier de sortie
        #[arg(short, long, default_value = "compressed.pdf")]
        output: PathBuf,
    },
    /// Convertir des images (JPG/PNG) en PDF
    ImagesToPdf {
        /// Images d'entrée
        #[arg(required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        /// Fichier de sortie
        #[arg(short, long, default_value = "images.pdf")]
        output: PathBuf,
    },
    /// Convertir un PDF en images JPG
    PdfToJpg {
        /// Fichier PDF d'entrée
        input: PathBuf,
        /// Répertoire de sortie
        #[arg(short, long, default_value = "pdf_jpg")]
        output_dir: PathBuf,
        /// Résolution en DPI (défaut 150)
        #[arg(long, default_value_t = 150)]
        dpi: u32,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Merge { inputs, output } => commands::merge(&inputs, &output),
        Commands::Split {
            input,
            output_dir,
            page,
        } => commands::split(&input, &output_dir, page),
        Commands::Compress { input, output } => commands::compress(&input, &output),
        Commands::ImagesToPdf { inputs, output } => commands::images_to_pdf(&inputs, &output),
        Commands::PdfToJpg {
            input,
            output_dir,
            dpi,
        } => commands::pdf_to_jpg(&input, &output_dir, dpi),
    };

    if let Err(e) = result {
        eprintln!("Erreur : {e}");
        std::process::exit(1);
    }
}
