use mlua::{Lua, Table, Value};
use phenix_client_acp::application_descriptor;

fn table_len(table: &Table) -> usize {
    table.pairs::<Value, Value>().count()
}

#[test]
fn generated_lua_descriptor_matches_the_fixed_application_descriptor() {
    let descriptor = application_descriptor();
    let source =
        phenix_binding_generator::lua(&descriptor).expect("fixed descriptor generates Lua");
    let lua = Lua::new();
    let generated: Table = lua.load(source).eval().expect("generated Lua evaluates");

    assert_eq!(
        generated
            .get::<String>("interface_id")
            .expect("generated interface id"),
        descriptor.id.as_str()
    );

    let capabilities = generated
        .get::<Table>("capabilities")
        .expect("generated capabilities");
    let types = generated.get::<Table>("types").expect("generated types");
    let operations = generated
        .get::<Table>("operations")
        .expect("generated operations");
    let events = generated.get::<Table>("events").expect("generated events");
    let callbacks = generated
        .get::<Table>("callbacks")
        .expect("generated callbacks");

    assert_eq!(table_len(&capabilities), descriptor.capabilities.len());
    assert_eq!(table_len(&types), descriptor.types.len());
    assert_eq!(table_len(&operations), descriptor.operations.len());
    assert_eq!(table_len(&events), descriptor.events.len());
    assert_eq!(table_len(&callbacks), descriptor.callbacks.len());

    let conversions = generated
        .get::<Table>("conversions")
        .expect("generated conversion index");
    let operation_conversions = conversions
        .get::<Table>("operations")
        .expect("operation conversions");
    let event_conversions = conversions
        .get::<Table>("events")
        .expect("event conversions");
    let callback_conversions = conversions
        .get::<Table>("callbacks")
        .expect("callback conversions");

    for (id, operation) in &descriptor.operations {
        let generated_operation = operations
            .get::<Table>(id.as_str())
            .expect("generated operation metadata");
        assert_eq!(
            generated_operation
                .get::<String>("capability")
                .expect("operation capability"),
            operation.capability.as_str()
        );
        assert_eq!(
            generated_operation
                .get::<String>("input")
                .expect("operation input"),
            operation.input.as_str()
        );
        assert_eq!(
            generated_operation
                .get::<String>("output")
                .expect("operation output"),
            operation.output.as_str()
        );
        assert_eq!(
            generated_operation
                .get::<String>("error")
                .expect("operation error"),
            operation.error.as_str()
        );

        let generated_conversion = operation_conversions
            .get::<Table>(id.as_str())
            .expect("generated operation conversion");
        assert_eq!(
            generated_conversion
                .get::<String>("request")
                .expect("request schema"),
            operation.input.as_str()
        );
        assert_eq!(
            generated_conversion
                .get::<String>("result")
                .expect("result schema"),
            operation.output.as_str()
        );
        assert_eq!(
            generated_conversion
                .get::<String>("error")
                .expect("error schema"),
            operation.error.as_str()
        );
    }

    for (id, event) in &descriptor.events {
        let generated_event = events
            .get::<Table>(id.as_str())
            .expect("generated event metadata");
        assert_eq!(
            generated_event
                .get::<String>("capability")
                .expect("event capability"),
            event.capability.as_str()
        );
        assert_eq!(
            generated_event
                .get::<String>("payload")
                .expect("event payload"),
            event.payload.as_str()
        );
        assert_eq!(
            event_conversions
                .get::<Table>(id.as_str())
                .expect("generated event conversion")
                .get::<String>("payload")
                .expect("event payload schema"),
            event.payload.as_str()
        );
    }

    for (id, callback) in &descriptor.callbacks {
        let generated_callback = callbacks
            .get::<Table>(id.as_str())
            .expect("generated callback metadata");
        assert_eq!(
            generated_callback
                .get::<String>("capability")
                .expect("callback capability"),
            callback.capability.as_str()
        );
        assert_eq!(
            generated_callback
                .get::<String>("request")
                .expect("callback request"),
            callback.request.as_str()
        );
        assert_eq!(
            generated_callback
                .get::<String>("response")
                .expect("callback response"),
            callback.response.as_str()
        );

        let generated_conversion = callback_conversions
            .get::<Table>(id.as_str())
            .expect("generated callback conversion");
        assert_eq!(
            generated_conversion
                .get::<String>("request")
                .expect("callback request schema"),
            callback.request.as_str()
        );
        assert_eq!(
            generated_conversion
                .get::<String>("response")
                .expect("callback response schema"),
            callback.response.as_str()
        );
    }

    let errors = generated
        .get::<Table>("error_variants")
        .expect("generated application error variants");
    assert!(errors
        .contains_key("cancelled")
        .expect("cancelled error key"));
    assert!(errors
        .contains_key("unsupported_capability")
        .expect("unsupported capability error key"));
}
