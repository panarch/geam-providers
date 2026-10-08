#!/usr/bin/env escript
%% The Rust consumer supplies messages and bytes produced by the actual provider.
%% Compare decompressed messages, never the compressors' wire bytes.
main([Directory, ResetText]) ->
    Reset = list_to_existing_atom(ResetText),
    Encoder = zlib:open(),
    Decoder = zlib:open(),
    ok = zlib:deflateInit(Encoder, default, deflated, -15, 8, default),
    ok = zlib:inflateInit(Decoder, -15),
    try
        lists:foreach(fun(Index) ->
            Base = filename:join(Directory, integer_to_list(Index)),
            {ok, Message} = file:read_file(Base ++ ".message"),
            {ok, ProviderWire} = file:read_file(Base ++ ".provider"),
            Message = iolist_to_binary(zlib:inflate(Decoder,
                <<ProviderWire/binary, 0, 0, 255, 255>>)),
            Full = iolist_to_binary(zlib:deflate(Encoder, Message, sync)),
            Size = byte_size(Full) - 4,
            <<Wire:Size/binary, 0, 0, 255, 255>> = Full,
            ok = file:write_file(Base ++ ".erlang", Wire),
            case Reset of
                true ->
                    ok = zlib:deflateReset(Encoder),
                    ok = zlib:inflateReset(Decoder);
                false -> ok
            end
        end, lists:seq(0, 3))
    after
        zlib:close(Encoder),
        zlib:close(Decoder)
    end,
    io:format("provider -> Erlang: four messages passed~n").
