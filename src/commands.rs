use std::fs;
use std::path::Path;

use image::ImageFormat;
use lopdf::{dictionary, Document, Object, ObjectId, Stream};

pub fn merge(inputs: &[std::path::PathBuf], output: &Path) -> Result<(), String> {
    if inputs.len() < 2 {
        return Err("Il faut au moins 2 fichiers PDF pour fusionner".into());
    }

    let mut document = Document::with_version("1.5");
    let mut max_id = 1u32;
    let mut documents_pages = std::collections::BTreeMap::new();
    let mut documents_objects = std::collections::BTreeMap::new();

    for input in inputs {
        let mut doc = Document::load(input).map_err(|e| format!("{} : {e}", input.display()))?;
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        for (_, object_id) in doc.get_pages() {
            let object = doc
                .get_object(object_id)
                .map_err(|e| format!("{} : {e}", input.display()))?
                .to_owned();
            documents_pages.insert(object_id, object);
        }
        documents_objects.extend(doc.objects);
    }

    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    for (object_id, object) in documents_objects.into_iter() {
        match object.type_name().unwrap_or("") {
            "Catalog" => {
                catalog_object = Some((
                    if let Some((id, _)) = catalog_object {
                        id
                    } else {
                        object_id
                    },
                    object,
                ));
            }
            "Pages" => {
                if let Ok(dictionary) = object.as_dict() {
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref object)) = pages_object {
                        if let Ok(old_dictionary) = object.as_dict() {
                            dictionary.extend(old_dictionary);
                        }
                    }
                    pages_object = Some((
                        if let Some((id, _)) = pages_object {
                            id
                        } else {
                            object_id
                        },
                        Object::Dictionary(dictionary),
                    ));
                }
            }
            "Page" => {}
            "Outlines" => {}
            "Outline" => {}
            _ => {
                document.objects.insert(object_id, object);
            }
        }
    }

    let (catalog_id, catalog_object) = catalog_object.ok_or("Catalog racine introuvable")?;
    let (page_id, page_object) = pages_object.ok_or("Pages racine introuvable")?;

    if let Ok(dictionary) = page_object.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Count", documents_pages.len() as u32);
        dictionary.set(
            "Kids",
            documents_pages
                .iter()
                .map(|(object_id, _)| Object::Reference(*object_id))
                .collect::<Vec<_>>(),
        );
        document.objects.insert(page_id, Object::Dictionary(dictionary));
    }

    for (object_id, object) in documents_pages.iter() {
        if let Ok(dictionary) = object.as_dict() {
            let mut dictionary = dictionary.clone();
            dictionary.set("Parent", page_id);
            document.objects.insert(*object_id, Object::Dictionary(dictionary));
        }
    }

    if let Ok(dictionary) = catalog_object.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Pages", page_id);
        dictionary.remove(b"Outlines");
        document.objects.insert(catalog_id, Object::Dictionary(dictionary));
    }

    document.trailer.set("Root", catalog_id);
    document.max_id = document.objects.len() as u32;
    document.renumber_objects();
    document
        .save(output)
        .map_err(|e| format!("écriture {} : {e}", output.display()))?;

    println!("Fusionné {} fichiers -> {}", inputs.len(), output.display());
    Ok(())
}

pub fn split(input: &Path, output_dir: &Path, page: Option<u32>) -> Result<(), String> {
    let doc = Document::load(input).map_err(|e| format!("{} : {e}", input.display()))?;
    let total = doc.get_pages().len();
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("création {} : {e}", output_dir.display()))?;

    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "page".into());

    let targets: Vec<u32> = match page {
        Some(p) => {
            if p == 0 || p as usize > total {
                return Err(format!("Page {p} hors limites (1..={total})"));
            }
            vec![p]
        }
        None => (1..=total as u32).collect(),
    };

    let target_count = targets.len();
    for p in targets {
        let mut single = Document::load(input).map_err(|e| format!("{} : {e}", input.display()))?;
        let to_delete: Vec<u32> = (1..=total as u32).filter(|&n| n != p).collect();
        single.delete_pages(&to_delete);
        let out = output_dir.join(format!("{stem}_page_{p}.pdf"));
        single
            .save(&out)
            .map_err(|e| format!("écriture {} : {e}", out.display()))?;
        println!("Page {p} -> {}", out.display());
    }
    println!("{} page(s) extraite(s) dans {}", target_count, output_dir.display());
    Ok(())
}

pub fn compress(input: &Path, output: &Path) -> Result<(), String> {
    let mut doc = Document::load(input).map_err(|e| format!("{} : {e}", input.display()))?;
    doc.compress();
    doc.save(output).map_err(|e| format!("écriture {} : {e}", output.display()))?;
    let before = fs::metadata(input).map(|m| m.len()).unwrap_or(0);
    let after = fs::metadata(output).map(|m| m.len()).unwrap_or(0);
    let pct = if before > 0 {
        (100.0 * (before as f64 - after as f64) / before as f64).max(0.0)
    } else {
        0.0
    };
    println!(
        "Compressé {} -> {} ({:.1}% gagné)",
        input.display(),
        output.display(),
        pct
    );
    Ok(())
}

pub fn images_to_pdf(inputs: &[std::path::PathBuf], output: &Path) -> Result<(), String> {
    if inputs.is_empty() {
        return Err("Aucune image fournie".into());
    }

    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut page_ids = Vec::new();

    for input in inputs {
        let (xobj_id, w, h) = image_xobject(&mut doc, input)?;
        // Flux de contenu qui dessine l'image sur toute la page.
        let content = format!("q {} 0 0 {} 0 0 cm /Im0 Do Q", w, h).into_bytes();
        let content_id = doc.add_object(Stream::new(dictionary! {}, content));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), w.into(), h.into()],
            "Resources" => dictionary! {
                "XObject" => dictionary! { "Im0" => xobj_id },
            },
            "Contents" => content_id,
        });
        page_ids.push(page_id);
    }

    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => page_ids.iter().map(|id| Object::Reference(*id)).collect::<Vec<_>>(),
        "Count" => page_ids.len() as u32,
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    doc.compress();
    doc.save(output).map_err(|e| format!("écriture {} : {e}", output.display()))?;

    println!("{} images -> {}", inputs.len(), output.display());
    Ok(())
}

fn image_xobject(doc: &mut Document, input: &Path) -> Result<(ObjectId, u32, u32), String> {
    let img = image::open(input).map_err(|e| format!("{} : {e}", input.display()))?;
    let (w, h) = (img.width(), img.height());
    let format = image::ImageReader::open(input)
        .map_err(|e| format!("{} : {e}", input.display()))?
        .with_guessed_format()
        .map_err(|e| format!("{} : {e}", input.display()))?
        .format();

    let (data, filter) = match format {
        Some(ImageFormat::Jpeg) => {
            let raw = fs::read(input).map_err(|e| format!("{} : {e}", input.display()))?;
            (raw, Some("DCTDecode"))
        }
        _ => {
            let rgb = img.to_rgb8();
            (rgb.into_raw(), None)
        }
    };

    let mut dict = dictionary! {
        "Type" => "XObject",
        "Subtype" => "Image",
        "Width" => w as u32,
        "Height" => h as u32,
        "ColorSpace" => "DeviceRGB",
        "BitsPerComponent" => 8,
    };
    if let Some(f) = filter {
        dict.set("Filter", f);
    }
    let mut stream = Stream::new(dict, data);
    // Ne compresse que les non-JPEG (FlateDecode). Les JPEG restent en DCTDecode.
    if filter.is_none() {
        stream
            .compress()
            .map_err(|e| format!("compression image : {e}"))?;
    }
    let id = doc.add_object(Object::Stream(stream));
    Ok((id, w, h))
}

pub fn pdf_to_jpg(input: &Path, output_dir: &Path, dpi: u32) -> Result<(), String> {
    use pdf_render::pdf_interpret::InterpreterSettings;
    use pdf_render::pdf_syntax::Pdf;
    use pdf_render::render_pdf;

    let data = fs::read(input).map_err(|e| format!("{} : {e}", input.display()))?;
    let pdf = Pdf::new(data).map_err(|e| format!("{} : {e:?}", input.display()))?;
    let page_count = pdf.pages().len();
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("création {} : {e}", output_dir.display()))?;

    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "page".into());

    let scale = dpi as f32 / 72.0;
    let pixmaps = render_pdf(&pdf, scale, InterpreterSettings::default(), None)
        .ok_or_else(|| "échec du rendu du PDF".to_string())?;

    for (i, pixmap) in pixmaps.into_iter().enumerate() {
        let (w, h) = (pixmap.width() as u32, pixmap.height() as u32);
        let rgba = pixmap.data_as_u8_slice();
        let img = image::RgbaImage::from_raw(w, h, rgba.to_vec())
            .ok_or_else(|| "taille d'image invalide".to_string())?;
        let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
        let out = output_dir.join(format!("{stem}_page_{}.jpg", i + 1));
        rgb.save_with_format(&out, ImageFormat::Jpeg)
            .map_err(|e| format!("écriture {} : {e}", out.display()))?;
        println!("Page {} -> {}", i + 1, out.display());
    }
    println!("{} pages rendues dans {}", page_count, output_dir.display());
    Ok(())
}

pub fn edit(input: &Path) -> Result<(), String> {
    if !input.exists() {
        return Err(format!("{} : fichier introuvable", input.display()));
    }

    // Vérifie que xournalpp est installé.
    let available = std::process::Command::new("xournalpp")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok();

    if !available {
        println!("xournalpp n'est pas installé.");
        if let Some(cmd) = install_command() {
            println!("Pour l'installer : {cmd}");
            print!("Voulez-vous l'installer maintenant ? [y/N] ");
            use std::io::Write;
            std::io::stdout().flush().map_err(|e| e.to_string())?;
            let mut answer = String::new();
            std::io::stdin()
                .read_line(&mut answer)
                .map_err(|e| e.to_string())?;
            if answer.trim().eq_ignore_ascii_case("y") {
                let status = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .status()
                    .map_err(|e| format!("lancement de l'installation : {e}"))?;
                if !status.success() {
                    return Err(format!(
                        "l'installation a échoué (code {:?}). Installez xournalpp manuellement.",
                        status.code()
                    ));
                }
            } else {
                return Err("Installation annulée. Installez xournalpp puis réessayez.".into());
            }
        } else {
            return Err(
                "Gestionnaire de paquets non reconnu. Installez xournalpp manuellement.".into(),
            );
        }
    }

    let status = std::process::Command::new("xournalpp")
        .arg(input)
        .status()
        .map_err(|e| format!("lancement de xournalpp : {e}"))?;

    if !status.success() {
        return Err(format!("xournalpp s'est terminé avec le code {:?}", status.code()));
    }
    Ok(())
}

/// Retourne la commande d'installation de xournalpp selon le gestionnaire de paquets détecté.
fn install_command() -> Option<String> {
    let candidates: &[(&str, &str)] = &[
        ("apt-get", "sudo apt-get install -y xournalpp"),
        ("dnf", "sudo dnf install -y xournalpp"),
        ("yum", "sudo yum install -y xournalpp"),
        ("pacman", "sudo pacman -S --noconfirm xournalpp"),
        ("zypper", "sudo zypper install -y xournalpp"),
        ("brew", "brew install --cask xournal"),
    ];
    for (bin, cmd) in candidates {
        if std::process::Command::new(bin)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
        {
            return Some((*cmd).to_string());
        }
    }
    None
}
