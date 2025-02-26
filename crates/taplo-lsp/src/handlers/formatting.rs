use lsp_async_stub::{rpc::Error, util::LspExt, Context, Params};
use lsp_types::{DocumentFormattingParams, TextEdit};
use std::path::PathBuf;
use taplo::formatter;
use taplo_common::{environment::Environment, util::Normalize};

use crate::World;

#[tracing::instrument(skip_all)]
pub(crate) async fn format<E: Environment>(
    context: Context<World<E>>,
    params: Params<DocumentFormattingParams>,
) -> Result<Option<Vec<TextEdit>>, Error> {
    let p = params.required()?;

    let workspaces = context.workspaces.read().await;
    let ws = workspaces.by_document(&p.text_document.uri);
    let doc = ws.document(&p.text_document.uri)?;

    let doc_path = PathBuf::from(p.text_document.uri.as_str()).normalize();

    let mut format_opts = formatter::Options {
        indent_string: if p.options.insert_spaces {
            " ".repeat(p.options.tab_size as usize)
        } else {
            "\t".into()
        },
        ..Default::default()
    };

    if let Some(v) = p.options.insert_final_newline {
        format_opts.trailing_newline = v;
    }

    format_opts.update_camel(ws.config.formatter.clone());

    ws.taplo_config
        .update_format_options(&doc_path, &mut format_opts);

    Ok(Some(vec![TextEdit {
        range: doc.mapper.all_range().into_lsp(),
        new_text: taplo::formatter::format_with_path_scopes(
            doc.dom.clone(),
            format_opts,
            &doc.parse
                .errors
                .iter()
                .map(|err| err.range)
                .collect::<Vec<_>>(),
            ws.taplo_config.format_scopes(&doc_path),
        )
        .map_err(|err| {
            tracing::error!(error = %err, "invalid key pattern");
            Error::internal_error().with_data("invalid Taplo configuration")
        })?,
    }]))
}
