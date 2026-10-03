use crate::deck::{resolve_media, Deck};
use std::io::Write;
use std::path::{Path, PathBuf};

fn convert_to_mp4(src: &Path) -> Option<PathBuf> {
    let ff = crate::export::find_ffmpeg()?;
    let out = std::env::temp_dir().join(format!(
        "keynote-conv-{}.mp4",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let st = std::process::Command::new(ff)
        .args([
            "-y",
            "-loglevel",
            "error",
            "-i",
            &src.display().to_string(),
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            &out.display().to_string(),
        ])
        .status()
        .ok()?;
    if st.success() && out.exists() {
        Some(out)
    } else {
        None
    }
}

struct SlideAsset {
    png: PathBuf,
    video: Option<PathBuf>,
    is_video_slide: bool,
}

/// Render every slide to PNG (poster) and collect embeddable videos.
fn render_assets(deck: &Deck, base_dir: &Path, width: u32) -> Result<Vec<SlideAsset>, String> {
    let mut out = Vec::new();
    for s in &deck.slides {
        let html = crate::export::slide_html(deck, base_dir, s.index, None, None);
        let tmp_html = std::env::temp_dir().join(format!("keynote-pptx-{}.html", s.index));
        std::fs::write(&tmp_html, html).map_err(|e| format!("write tmp: {e}"))?;
        let png = std::env::temp_dir().join(format!("keynote-pptx-{}-{width}.png", s.index));
        crate::export::screenshot_png(&tmp_html, &png, width)?;
        let mut video: Option<PathBuf> = None;
        if let Some(m) = s.media.first() {
            if m.is_video() {
                if let Some(resolved) = resolve_media(base_dir, &m.src) {
                    let ext = resolved
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    if ext == "mp4" {
                        video = Some(resolved);
                    } else if let Some(mp4) = convert_to_mp4(&resolved) {
                        video = Some(mp4);
                    }
                }
            } else if m.is_animated() {
                // Hype: animated WebP/GIF become MP4 in PPTX.
                if let Some(resolved) = resolve_media(base_dir, &m.src) {
                    if let Some(mp4) = convert_to_mp4(&resolved) {
                        video = Some(mp4);
                    }
                }
            }
        }
        let is_video_slide = video.is_some();
        out.push(SlideAsset { png, video, is_video_slide });
    }
    Ok(out)
}

const EMU_X: u32 = 9144000; // 10in
const EMU_Y: u32 = 5143500; // 5.625in (16:9)

fn slide_xml(_idx: usize, has_video: bool) -> String {
    let pic_id = 2u32;
    let video_id = 3u32;
    let video_shape = if has_video {
        format!(
            r#"<p:nvPr><a:videoFile r:embed="rId3"/></p:nvPr>"#
        )
        .replace("rId3", &format!("rId{}", video_id))
        .replace("<p:nvPr>", &format!("<p:cNvPr id=\"{video_id}\" name=\"Video\"/><p:cNvVideoFile/><p:nvPr>"))
        // Build a proper video pic element below instead; keep placeholder simple:
        .replace("placeholder", "x")
    } else {
        String::new()
    };
    let _ = video_shape;
    // Full-slide picture; for video slides also embed a media element referencing the video part.
    let video_el = if has_video {
        format!(
            r#"<p:pic><p:nvPicPr><p:cNvPr id="{video_id}" name="VideoPoster"/><p:cNvPicPr/><p:nvPr><a:videoFile r:embed="rId{video_id}"/></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="rId{pic_id}"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{EMU_X}" cy="{EMU_Y}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></a:spPr></p:pic>"#
        )
    } else {
        String::new()
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{EMU_X}" cy="{EMU_Y}"/><a:chOff x="0" y="0"/><a:chExt cx="{EMU_X}" cy="{EMU_Y}"/></a:xfrm></p:grpSpPr><p:pic><p:nvPicPr><p:cNvPr id="{pic_id}" name="Slide"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId{pic_id}"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{EMU_X}" cy="{EMU_Y}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></a:spPr></p:pic>{video_el}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"#,
        pic_id = pic_id,
        video_el = video_el,
        EMU_X = EMU_X,
        EMU_Y = EMU_Y,
    )
}

pub fn export_pptx(deck: &Deck, base_dir: &Path, width: u32) -> Result<Vec<u8>, String> {
    let assets = render_assets(deck, base_dir, width)?;
    let title = deck
        .frontmatter
        .title
        .clone()
        .or_else(|| deck.slides.first().and_then(|s| s.title.clone()))
        .unwrap_or_else(|| "keynote".into());

    let mut buf = std::io::Cursor::new(Vec::<u8>::new());
    let mut zip = zip::ZipWriter::new(&mut buf);
    let opt = zip::write::SimpleFileOptions::default();

    // Collect media names first (imageN.png, videoN.mp4).
    let mut image_names: Vec<String> = Vec::new();
    let mut video_names: Vec<Option<String>> = Vec::new();
    for (i, a) in assets.iter().enumerate() {
        image_names.push(format!("image{}.png", i + 1));
        if let Some(v) = &a.video {
            let ext = v
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_else(|| "mp4".into());
            video_names.push(Some(format!("video{}.{ext}", i + 1)));
        } else {
            video_names.push(None);
        }
    }

    // [Content_Types].xml
    {
        let mut types = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Default Extension="jpg" ContentType="image/jpeg"/><Default Extension="mp4" ContentType="video/mp4"/><Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/><Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/><Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>"#,
        );
        for i in 0..assets.len() {
            types.push_str(&format!(r#"<Override PartName="/ppt/slides/slide{}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#, i + 1));
        }
        types.push_str("</Types>");
        zip.start_file("[Content_Types].xml", opt).map_err(|e| e.to_string())?;
        zip.write_all(types.as_bytes()).map_err(|e| e.to_string())?;
    }
    // _rels/.rels
    zip.start_file("_rels/.rels", opt).map_err(|e| e.to_string())?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>"#).map_err(|e| e.to_string())?;
    // docProps
    zip.start_file("docProps/core.xml", opt).map_err(|e| e.to_string())?;
    zip.write_all(format!(r#"<?xml version="1.0" encoding="UTF-8"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>{title}</dc:title><dc:creator>keynote</dc:creator></cp:coreProperties>"#, title = xml_escape(&title)).as_bytes()).map_err(|e| e.to_string())?;
    zip.start_file("docProps/app.xml", opt).map_err(|e| e.to_string())?;
    zip.write_all(format!(r#"<?xml version="1.0" encoding="UTF-8"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>keynote</Application><Slides>{}</Slides></Properties>"#, assets.len()).as_bytes()).map_err(|e| e.to_string())?;
    // presentation.xml
    {
        let mut sld_ids = String::new();
        for i in 0..assets.len() {
            sld_ids.push_str(&format!(r#"<p:sldId id="{}" r:id="rId{}"/>"#, 256 + i, i + 2));
        }
        let pres = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst>{sld_ids}</p:sldIdLst><p:sldSz cx="{EMU_X}" cy="{EMU_Y}"/></p:presentation>"#);
        zip.start_file("ppt/presentation.xml", opt).map_err(|e| e.to_string())?;
        zip.write_all(pres.as_bytes()).map_err(|e| e.to_string())?;
        // presentation rels
        let mut rels = String::from(r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>"#);
        for i in 0..assets.len() {
            rels.push_str(&format!(r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>"#, i + 2, i + 1));
        }
        rels.push_str("</Relationships>");
        zip.start_file("ppt/_rels/presentation.xml.rels", opt).map_err(|e| e.to_string())?;
        zip.write_all(rels.as_bytes()).map_err(|e| e.to_string())?;
    }
    // master + layout (minimal blank)
    zip.start_file("ppt/slideMasters/slideMaster1.xml", opt).map_err(|e| e.to_string())?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:bg><p:bgPr><a:solidFill><a:srgbClr val="111111"/></a:solidFill><a:effectLst/></p:bgPr></p:bg><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="9144000" cy="5143500"/><a:chOff x="0" y="0"/><a:chExt cx="9144000" cy="5143500"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld><p:sldLayoutIdLst><p:sldLayoutId r:id="rId1"/></p:sldLayoutIdLst></p:sldMaster>"#).map_err(|e| e.to_string())?;
    zip.start_file("ppt/slideMasters/_rels/slideMaster1.xml.rels", opt).map_err(|e| e.to_string())?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/></Relationships>"#).map_err(|e| e.to_string())?;
    zip.start_file("ppt/slideLayouts/slideLayout1.xml", opt).map_err(|e| e.to_string())?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="blank"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="9144000" cy="5143500"/><a:chOff x="0" y="0"/><a:chExt cx="9144000" cy="5143500"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld></p:sldLayout>"#).map_err(|e| e.to_string())?;

    // slides + rels + media
    for (i, a) in assets.iter().enumerate() {
        let n = i + 1;
        let has_video = a.video.is_some();
        let xml = slide_xml(i, has_video);
        zip.start_file(format!("ppt/slides/slide{n}.xml"), opt).map_err(|e| e.to_string())?;
        zip.write_all(xml.as_bytes()).map_err(|e| e.to_string())?;
        // rels with concrete media names
        let mut rels = format!(r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/{}"/>"#, image_names[i]);
        if let Some(vn) = &video_names[i] {
            rels.push_str(&format!(r#"<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/video" Target="../media/{vn}"/>"#));
        }
        rels.push_str("</Relationships>");
        zip.start_file(format!("ppt/slides/_rels/slide{n}.xml.rels"), opt).map_err(|e| e.to_string())?;
        zip.write_all(rels.as_bytes()).map_err(|e| e.to_string())?;
        // media bytes
        let png_bytes = std::fs::read(&a.png).map_err(|e| format!("read png: {e}"))?;
        zip.start_file(format!("ppt/media/{}", image_names[i]), opt).map_err(|e| e.to_string())?;
        zip.write_all(&png_bytes).map_err(|e| e.to_string())?;
        if let (Some(src), Some(vn)) = (&a.video, &video_names[i]) {
            if let Ok(vb) = std::fs::read(src) {
                zip.start_file(format!("ppt/media/{vn}"), opt).map_err(|e| e.to_string())?;
                zip.write_all(&vb).map_err(|e| e.to_string())?;
            }
        }
        let _ = a.is_video_slide;
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(buf.into_inner())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
