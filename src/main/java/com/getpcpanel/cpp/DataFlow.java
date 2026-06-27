package com.getpcpanel.cpp;

import lombok.Getter;
import lombok.RequiredArgsConstructor;
import lombok.experimental.Accessors;

@Getter
@Accessors(fluent = true)
@RequiredArgsConstructor
public enum DataFlow {
    dfRender(false, true), dfCapture(true, false), dfAll(true, true);
    private final boolean input;
    private final boolean output;

    public static DataFlow from(int ord) {
        return values()[ord];
    }
}
